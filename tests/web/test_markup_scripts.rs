// `js{…}` is the element script, and a script's content is JavaScript as written, unescaped (card js-element)
use crate::is;

#[test]
fn js_is_the_script_element() {
	is!("use markup; to_html(js{\"alert(1)\"})", "<script>alert(1)</script>");
	is!("use markup; to_html(html{js{\"alert('ok')\"}})", "<html><script>alert('ok')</script></html>");
}

#[test]
fn script_content_is_not_escaped() {
	is!("use markup; to_html(script{\"if (a < b && c > d) go()\"})", "<script>if (a < b && c > d) go()</script>");
	is!("use markup; to_html(script{\"s = '</script>'\"})", "<script>s = '<\\/script>'</script>");
}
