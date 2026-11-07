// libwarp_webview (card app-webview, decision native-js-webview): the system's web view for the native warp, on macOS
// WKWebView. It answers the host word foreign_call for `use js` (bound by lib/native/foreign.c library_word) in one
// hidden web view per run, made at the first call: never shown, never activating the app, no Dock icon. And it shows a
// page's HTML in a window for `warp app` (lib/app/command.warp, lib/webview.warp), never under WARP_NO_WINDOW or CI.
// Built by scripts/wasmtime-c-api.sh
#import <Cocoa/Cocoa.h>
#import <WebKit/WebKit.h>
#include "warp_json.h"

// a secure origin, so the page's crypto.randomUUID and the like are there
static NSString* const PAGE_ORIGIN = @"https://localhost/";
static NSString* const EMPTY_PAGE = @"<!doctype html><meta charset=utf-8><title>warp</title>";
static const NSTimeInterval ANSWER_SECONDS = 10, SPIN_SECONDS = 0.005;
enum { WINDOW_WIDTH = 900, WINDOW_HEIGHT = 700 };
static const char* const JS_RUNTIME = "js";
// web/playground/js-members.js (jsMember, shared with the page) and bridge.js (warpForeignCall), put into each page
static const char BRIDGE[] = {
#embed "../../web/playground/js-members.js"
	, '\n',
#embed "bridge.js"
	, 0};
// a module's or member's name as `use js` writes it, never JS source (an argument crosses only as JSON)
static NSString* const NAME_PATTERN = @"^[A-Za-z_$][A-Za-z0-9_$]*$";

static int windows_refused(void) { return getenv("WARP_NO_WINDOW") || getenv("CI"); }

// the run loop turned until done() or the seconds are over: NO when they were
static BOOL spun(BOOL (^done)(void), NSTimeInterval seconds) {
	NSDate* until = [NSDate dateWithTimeIntervalSinceNow:seconds];
	while (!done() && until.timeIntervalSinceNow > 0)
		[NSRunLoop.currentRunLoop runMode:NSDefaultRunLoopMode beforeDate:[NSDate dateWithTimeIntervalSinceNow:SPIN_SECONDS]];
	return done();
}

@interface WarpLoading : NSObject <WKNavigationDelegate>
@property int loaded;
@end
@implementation WarpLoading
- (void)webView:(WKWebView*)view didFinishNavigation:(WKNavigation*)navigation { self.loaded = 1; }
- (void)webView:(WKWebView*)view didFailNavigation:(WKNavigation*)navigation withError:(NSError*)error { self.loaded = 1; }
@end

static WKWebView* view_made(NSRect frame) {
	WKWebViewConfiguration* configuration = [WKWebViewConfiguration new];
	WKUserScript* bridge = [[WKUserScript alloc] initWithSource:@(BRIDGE) injectionTime:WKUserScriptInjectionTimeAtDocumentStart forMainFrameOnly:YES];
	[configuration.userContentController addUserScript:bridge];
	return [[WKWebView alloc] initWithFrame:frame configuration:configuration];
}
// the HTML shown in the view, waited for: 0 when it did not load in time
static BOOL page_loaded(WKWebView* view, NSString* html) {
	WarpLoading* loading = [WarpLoading new];
	view.navigationDelegate = loading;
	[view loadHTMLString:html baseURL:[NSURL URLWithString:PAGE_ORIGIN]];
	BOOL loaded = spun(^{ return (BOOL)loading.loaded; }, ANSWER_SECONDS);
	view.navigationDelegate = nil;
	return loaded;
}
static void application_made(NSApplicationActivationPolicy policy) {
	[NSApplication sharedApplication];
	if (NSApp.activationPolicy == NSApplicationActivationPolicyProhibited || policy != NSApplicationActivationPolicyProhibited) [NSApp setActivationPolicy:policy];
}
// the run's hidden web view, made at its first use: the process never becomes an app on screen for it
static WKWebView* hidden_view(void) {
	static WKWebView* view;
	if (view) return view;
	application_made(NSApplicationActivationPolicyProhibited);
	view = view_made(NSMakeRect(0, 0, 1, 1));
	if (!page_loaded(view, EMPTY_PAGE)) fprintf(stderr, "libwarp_webview: the hidden web view did not load\n");
	return view;
}

// the body run as an async function in the view with these arguments: its answer, or nil and the failure in *failure
static NSString* answered(WKWebView* view, NSString* body, NSDictionary* arguments, NSString** failure) {
	__block BOOL done = NO;
	__block id answer = nil;
	__block NSString* failed = nil;
	[view callAsyncJavaScript:body arguments:arguments inFrame:nil inContentWorld:WKContentWorld.pageWorld completionHandler:^(id result, NSError* error) {
		answer = result;
		failed = error ? (error.userInfo[@"WKJavaScriptExceptionMessage"] ?: error.localizedDescription) : nil;
		done = YES;
	}];
	if (!spun(^{ return done; }, ANSWER_SECONDS)) failed = [NSString stringWithFormat:@"no answer in %g s (a promise that never settles?)", ANSWER_SECONDS];
	*failure = failed;
	return failed ? nil : [answer isKindOfClass:NSString.class] ? answer : [answer description];
}

static int is_name(NSString* name) { return [name rangeOfString:NAME_PATTERN options:NSRegularExpressionSearch].location != NSNotFound; }
static NSString* node_text(warp_values* values, const wasmtime_anyref_t* node) {
	char* bytes = warp_text_bytes(values, node, NULL);
	NSString* text = @(bytes) ?: @"";
	free(bytes);
	return text;
}
// the call's JSON {module, member, arguments}: the module a name or a handle's value, the arguments null for a read
static NSString* request_of(warp_values* values, const wasmtime_val_t* given, NSString* member) {
	json_text json = {0};
	json_said(&json, "{\"module\":");
	json_of_node(values, &json, &given[1].of.anyref);
	json_said(&json, ",\"member\":");
	json_string(&json, member.UTF8String, strlen(member.UTF8String));
	json_said(&json, ",\"arguments\":");
	if (warp_node_int(values, &given[3].of.anyref) == 1) json_of_node(values, &json, &given[4].of.anyref); else json_said(&json, "null");
	json_said(&json, "}");
	NSString* request = @(json.bytes);
	free(json.bytes);
	return request;
}
// a member `use js` may name: an identifier, or as js-members.js reads it "" (construct), "set x", "a.b"
static int member_named(NSString* member) {
	NSString* name = [member hasPrefix:@"set "] ? [member substringFromIndex:4] : member;
	if (name.length == 0) return 1;
	for (NSString* part in [name componentsSeparatedByString:@"."]) if (!is_name(part)) return 0;
	return 1;
}

// the failure as the page says it: "js Math.nope: ReferenceError: …", the call named once, without WebKit's "Error: "
static NSString* failure_said(NSString* called, NSString* failure) {
	NSString* plain = [failure hasPrefix:@"Error: "] ? [failure substringFromIndex:7] : failure;
	return [plain hasPrefix:called] ? plain : [NSString stringWithFormat:@"%@: %@", called, plain];
}
// foreign_call(runtime, module, member, call, arguments), as the page's host-foreign.js foreignCall answers it
wasm_trap_t* warp_word_foreign_call(void* engine, wasmtime_caller_t* caller, const wasmtime_val_t* given, size_t given_count, wasmtime_val_t* results, size_t result_count) {
	(void)given_count, (void)result_count;
	@autoreleasepool {
		warp_values values = warp_values_of(engine, caller, &given[0].of.anyref);
		wasmtime_anyref_t answer;
		wasmtime_anyref_set_null(&answer);
		NSString* runtime = node_text(&values, &given[0].of.anyref);
		NSString* member = node_text(&values, &given[2].of.anyref);
		int named_module = warp_node_kind(&values, &given[1].of.anyref) != WARP_KIND_LIST;
		NSString* module = named_module ? node_text(&values, &given[1].of.anyref) : @"value";
		NSString* called = [NSString stringWithFormat:@"%@ %@.%@", runtime, module, member];
		NSString* failure = nil;
		if (strcmp(runtime.UTF8String, JS_RUNTIME)) failure = [NSString stringWithFormat:@"%@: %@ is not in the native warp yet", called, runtime];
		else if ((named_module && !is_name(module)) || !member_named(member)) failure = [NSString stringWithFormat:@"%@: not a name js has", called];
		NSString* request = failure || values.failure[0] ? nil : request_of(&values, given, member);
		NSString* json = request && !values.failure[0] ? answered(hidden_view(), @"return await warpForeignCall(request)", @{@"request": request}, &failure) : nil;
		if (failure) warp_failed(&values, failure_said(called, failure).UTF8String);
		else if (json) answer = node_of_json(&values, json.UTF8String);
		return warp_answered(&values, answer, results);
	}
}

// `warp app check`: the text the page shows, read in the hidden web view; the caller frees it
char* webview_page_text(const char* html) {
	@autoreleasepool {
		WKWebView* view = hidden_view();
		NSString* failure = nil;
		NSString* text = page_loaded(view, @(html)) ? answered(view, @"return document.body?.innerText ?? ''", @{}, &failure) : nil;
		if (!text) fprintf(stderr, "libwarp_webview: the page did not load%s%s\n", failure ? ": " : "", failure ? failure.UTF8String : "");
		return strdup(text ? text.UTF8String : "");
	}
}

@interface WarpWindowClosing : NSObject <NSWindowDelegate>
@end
@implementation WarpWindowClosing
- (void)windowWillClose:(NSNotification*)notification {
	[NSApp stop:nil];
	// stop takes effect after the next event
	[NSApp postEvent:[NSEvent otherEventWithType:NSEventTypeApplicationDefined location:NSZeroPoint modifierFlags:0 timestamp:0 windowNumber:0 context:nil subtype:0 data1:0 data2:0] atStart:NO];
}
@end

// `warp app`: the page's HTML in a window titled so, shown until it is closed; 1 refused (WARP_NO_WINDOW, CI)
int32_t webview_show(const char* html, const char* title) {
	if (windows_refused()) {
		fprintf(stderr, "warp app: no window under WARP_NO_WINDOW or CI\n");
		return 1;
	}
	@autoreleasepool {
		application_made(NSApplicationActivationPolicyAccessory);
		NSWindow* window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)
			styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskResizable | NSWindowStyleMaskMiniaturizable
			backing:NSBackingStoreBuffered defer:NO];
		window.releasedWhenClosed = NO;
		window.title = @(title);
		WarpWindowClosing* closing = [WarpWindowClosing new];
		window.delegate = closing;
		WKWebView* view = view_made(window.contentView.bounds);
		view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
		[window.contentView addSubview:view];
		[view loadHTMLString:@(html) baseURL:[NSURL URLWithString:PAGE_ORIGIN]];
		[window center];
		[window makeKeyAndOrderFront:nil];
		[NSApp activateIgnoringOtherApps:YES];
		[NSApp run];
		window.delegate = nil;
		return 0;
	}
}
