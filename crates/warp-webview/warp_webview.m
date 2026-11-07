// libwarp_webview (card app-webview, decision native-js-webview): the system's web view for the native warp, on macOS
// WKWebView. It answers the host word foreign_call for `use js` (bound by lib/native/foreign.c library_word) in one
// hidden web view per run, made at the first call: never shown, never activating the app, no Dock icon. `warp app` shows
// that view in a window, the program's handlers live (warpWindow below, lib/markup.warp page_window_run), never under
// WARP_NO_WINDOW or CI.
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
// `warp app` names the window so (lib/app/command.warp): the program's page shows there, live
static const char* const APP_WINDOW_VARIABLE = "WARP_APP_WINDOW";
// web/playground/js-members.js (jsMember), markup.js (listenToElements, morphChildren) and markup-forms.js (submit),
// shared with the page, and bridge.js (warpForeignCall, the page's events), put into each page
static const char BRIDGE[] = {
#embed "../../web/playground/js-members.js"
	, '\n',
#embed "../../web/playground/markup.js"
	, '\n',
#embed "../../web/playground/markup-forms.js"
	, '\n',
#embed "bridge.js"
	, 0};
// the module `use js` calls answered here, not in the page: the run's window, `show` it, the `next` event of its page
static NSString* const APP_WINDOW = @"warpWindow";
// what warpWindow.next answers once its time is over without an event (lib/markup.warp page_window_run)
static NSString* const TICK_EVENT = @"tick";
// the message bridge.js posts each event of the page with, its {event, detail} as JSON
static NSString* const EVENT_MESSAGE = @"warpEvent";
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

// the page's events not yet given to the program, each its {event, detail} as JSON
static NSMutableArray<NSString*>* page_events;
@interface WarpEvents : NSObject <WKScriptMessageHandler>
@end
@implementation WarpEvents
- (void)userContentController:(WKUserContentController*)controller didReceiveScriptMessage:(WKScriptMessage*)message {
	if ([message.body isKindOfClass:NSString.class]) [page_events addObject:message.body];
}
@end

static WKWebView* view_made(NSRect frame) {
	WKWebViewConfiguration* configuration = [WKWebViewConfiguration new];
	WKUserScript* bridge = [[WKUserScript alloc] initWithSource:@(BRIDGE) injectionTime:WKUserScriptInjectionTimeAtDocumentStart forMainFrameOnly:YES];
	[configuration.userContentController addUserScript:bridge];
	page_events = page_events ?: [NSMutableArray new];
	[configuration.userContentController addScriptMessageHandler:[WarpEvents new] name:EVENT_MESSAGE];
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
// the run's window, the hidden view in it: open until it is closed
static NSWindow* app_window;
static BOOL app_window_open;
@interface WarpAppWindowClosing : NSObject <NSWindowDelegate>
@end
@implementation WarpAppWindowClosing
- (void)windowWillClose:(NSNotification*)notification { app_window_open = NO; }
@end
static NSWindow* window_made(NSString* title, id<NSWindowDelegate> delegate) {
	NSWindow* window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)
		styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskResizable | NSWindowStyleMaskMiniaturizable
		backing:NSBackingStoreBuffered defer:NO];
	window.releasedWhenClosed = NO;
	window.title = title;
	window.delegate = delegate;
	return window;
}
static void window_shown(NSWindow* window, WKWebView* view) {
	view.frame = window.contentView.bounds;
	view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
	[window.contentView addSubview:view];
	[window center];
	[window makeKeyAndOrderFront:nil];
	[NSApp activateIgnoringOtherApps:YES];
}
// the app's events (clicks, keys) handed to its windows until done() or the seconds are over, as [NSApp run] would
static BOOL pumped(BOOL (^done)(void), NSTimeInterval seconds) {
	NSDate* until = [NSDate dateWithTimeIntervalSinceNow:seconds];
	while (!done() && until.timeIntervalSinceNow > 0) {
		NSEvent* event = [NSApp nextEventMatchingMask:NSEventMaskAny untilDate:[NSDate dateWithTimeIntervalSinceNow:SPIN_SECONDS] inMode:NSDefaultRunLoopMode dequeue:YES];
		if (event) [NSApp sendEvent:event];
	}
	return done();
}
// `warpWindow.show(title)`: the run's view in a window; `warpWindow.next(milliseconds)`: the page's next event,
// {event, detail, now} as JSON, else {event: "tick", now} once the milliseconds are over (none or -1: wait for an
// event), now the milliseconds since 1970; nil once the window is closed. *failure says why it can't
static NSString* window_answered(NSString* member, NSString* title, NSString* request, NSString** failure) {
	if ([member isEqualToString:@"show"]) {
		if (windows_refused()) *failure = @"no window under WARP_NO_WINDOW or CI";
		else if (!app_window) {
			WKWebView* view = hidden_view();
			application_made(NSApplicationActivationPolicyAccessory);
			[NSApp finishLaunching];
			app_window = window_made(title, [WarpAppWindowClosing new]);
			app_window_open = YES;
			window_shown(app_window, view);
		}
		return nil;
	}
	if (![member isEqualToString:@"next"]) {
		*failure = [NSString stringWithFormat:@"the window has no %@: show or next", member];
		return nil;
	}
	if (!app_window) {
		*failure = @"no window shown: warpWindow.show(title) first";
		return nil;
	}
	NSArray* given = [NSJSONSerialization JSONObjectWithData:[request dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil][@"arguments"];
	NSNumber* waited = [given isKindOfClass:NSArray.class] && [given.firstObject isKindOfClass:NSNumber.class] ? given.firstObject : nil;
	NSTimeInterval seconds = waited && waited.doubleValue >= 0 ? waited.doubleValue / 1000 : DBL_MAX;
	pumped(^{ return (BOOL)(page_events.count > 0 || !app_window_open); }, seconds);
	if (!app_window_open) return nil;
	NSMutableDictionary* happened = [NSMutableDictionary dictionaryWithDictionary:@{@"event": TICK_EVENT}];
	if (page_events.count) {
		[happened setDictionary:[NSJSONSerialization JSONObjectWithData:[page_events.firstObject dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil]];
		[page_events removeObjectAtIndex:0];
	}
	happened[@"now"] = @((long long)(NSDate.date.timeIntervalSince1970 * 1000));
	return [[NSString alloc] initWithData:[NSJSONSerialization dataWithJSONObject:happened options:0 error:nil] encoding:NSUTF8StringEncoding];
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
		BOOL windowed = !failure && named_module && [module isEqualToString:APP_WINDOW];
		NSString* request = failure || values.failure[0] ? nil : request_of(&values, given, member);
		NSString* json = windowed ? window_answered(member, @(getenv(APP_WINDOW_VARIABLE) ?: "warp"), request, &failure)
			: request && !values.failure[0] ? answered(hidden_view(), @"return await warpForeignCall(request)", @{@"request": request}, &failure) : nil;
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
