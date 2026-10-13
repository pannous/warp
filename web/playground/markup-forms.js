// Forms (a part of markup.js, card web-submit): loaded after markup.js only by pages whose program has an
// `on submit` handler (src/web/site.rs), and always by the playground.

// the events a site listens for besides site.js SITE_EVENTS
var FORM_EVENTS = { submit: submitDetail };

// a form sent: `form{ on submit { … } }` reads its named fields as event.fields (card web-submit), a number field's as
// a number and a checkbox's as checked or not; the form stays on the page, a form without the handler goes to its server
function submitDetail(submitted) {
	const { target: form, submitter } = submitted;
	if (form.hasAttribute("data-warp-submit")) submitted.preventDefault();
	const fields = Object.fromEntries(new FormData(form, submitter));
	for (const field of form.elements) {
		if (field.name && field.type === "checkbox") fields[field.name] = field.checked;
		else if (field.name && NUMBER_FIELDS.includes(field.type)) fields[field.name] = field.valueAsNumber;
	}
	return { fields };
}
