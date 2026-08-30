//! What a reader sees.
//!
//! **Every row in full.** MCF's own sharing surface prints the rows rather than
//! a count of them, because a person deciding about something irreversible is
//! entitled to read what it says. The same holds on the other side: somebody
//! looking at what was published should see what was published, not the site's
//! summary of it.
//!
//! **Nothing comes from anywhere.** No script, no stylesheet, no font: the page
//! is what the server sent, so a reader's browser makes one request and MCF's
//! archive has no third party in it.

use crate::archive::Held;

/// HTML-escapes text that came from somebody else.
///
/// Everything on these pages came from a file a stranger uploaded, so
/// everything on these pages goes through here.
fn safe(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(character),
        }
    }
    out
}

const STYLE: &str = "
:root{--ground:#f6f5f3;--panel:#fff;--ink:#17161a;--quiet:#5f5c58;--line:#e2dfd9;
--accent:#8a5a1c;--good:#2c6a4f;--bad:#93392c;
--mono:ui-monospace,'SF Mono',Menlo,Consolas,monospace}
@media(prefers-color-scheme:dark){:root{--ground:#121113;--panel:#1a191d;--ink:#e9e6e1;
--quiet:#97928a;--line:#2a2830;--accent:#d7a260;--good:#7cc39b;--bad:#dd8b78}}
*{box-sizing:border-box}
body{margin:0;background:var(--ground);color:var(--ink);line-height:1.6;
font-family:system-ui,-apple-system,'Segoe UI',sans-serif}
.wrap{max-width:60rem;margin:0 auto;padding:3rem 1.25rem 5rem}
h1{font-size:1.6rem;margin:0 0 .4rem;letter-spacing:-.01em}
h1 span{color:var(--accent)}
.lede{color:var(--quiet);margin:0 0 2rem;max-width:40rem}
h2{font-size:.75rem;text-transform:uppercase;letter-spacing:.09em;color:var(--quiet);
margin:2.5rem 0 .75rem;font-weight:600}
.card{background:var(--panel);border:1px solid var(--line);border-radius:9px;
padding:1rem 1.15rem;margin-bottom:.75rem}
.card a{color:inherit;text-decoration:none}
.arms{font-weight:600;margin-bottom:.3rem}
.meta{font-family:var(--mono);font-size:.78rem;color:var(--quiet)}
pre{font-family:var(--mono);font-size:.78rem;white-space:pre-wrap;word-break:break-word;
margin:0;line-height:1.7}
.row{border-left:3px solid var(--accent);padding-left:.9rem;margin-bottom:1rem}
.refusal{border-left:3px solid var(--bad);padding-left:.9rem;color:var(--bad)}
.empty{color:var(--quiet);font-style:italic}
code{font-family:var(--mono);font-size:.85em;background:var(--ground);padding:.1rem .35rem;
border-radius:3px}
footer{margin-top:3rem;padding-top:1.2rem;border-top:1px solid var(--line);
color:var(--quiet);font-size:.83rem}
";

fn shell(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{}</title><style>{STYLE}</style></head><body><div class=\"wrap\">{body}\
         <footer>Everything here was published by somebody who chose to. MCF cannot send: \
         it wrote a file and a person moved it. Publication cannot be undone, so nothing \
         here can be withdrawn.</footer></div></body></html>",
        safe(title)
    )
}

/// The list of everything held.
#[must_use]
pub fn index(held: &[Held]) -> String {
    let mut body = String::from(
        "<h1>MCF<span>.</span> contributions</h1>\
         <p class=\"lede\">Measurements people chose to publish. What travels is outcomes \
         only — scores, classifications, conditions and effect sizes. No prompt, no \
         completion, no task and no file.</p>",
    );
    if held.is_empty() {
        body.push_str("<p class=\"empty\">Nothing has been published yet.</p>");
    } else {
        body.push_str(&format!("<h2>{} published</h2>", held.len()));
        for one in held {
            let arms = one.arms();
            body.push_str(&format!(
                "<div class=\"card\"><a href=\"/c/{}\"><div class=\"arms\">{}</div>\
                 <div class=\"meta\">{} row(s) · {}</div></a></div>",
                safe(&one.digest),
                if arms.is_empty() {
                    "an unnamed measurement".to_owned()
                } else {
                    safe(&arms.join(" · "))
                },
                one.rows().len(),
                safe(&one.digest)
            ));
        }
    }
    body.push_str(
        "<h2>Publishing your own</h2>\
         <p class=\"lede\"><code>mcf share --into contribution.txt</code> writes the rows \
         MCF can contribute, showing you every one before it does. Sending the file is \
         your act, not MCF's.</p>",
    );
    shell("MCF contributions", &body)
}

/// One contribution, every row in full.
#[must_use]
pub fn one(held: &Held) -> String {
    let mut body = format!(
        "<h1>A contribution</h1><p class=\"lede\">{} row(s), as they were published.</p>",
        held.rows().len()
    );
    for row in held.rows() {
        body.push_str(&format!("<div class=\"row\"><pre>{}</pre></div>", safe(row)));
    }
    body.push_str(&format!(
        "<h2>As it arrived</h2><div class=\"card\"><pre>{}</pre></div>\
         <p class=\"lede\"><a href=\"/\">Everything published</a></p>",
        safe(&held.body)
    ));
    shell("A contribution", &body)
}

/// What a successful submission says.
#[must_use]
pub fn kept(digest: &str) -> String {
    shell(
        "Published",
        &format!(
            "<h1>Published</h1><p class=\"lede\">It is at <a href=\"/c/{0}\">/c/{0}</a>. \
             It cannot be withdrawn — MCF's terms said so before you sent it, and this is \
             where that becomes true.</p>",
            safe(digest)
        ),
    )
}

/// What a refusal says, in words rather than a code.
#[must_use]
pub fn refused(why: &str) -> String {
    shell(
        "Not published",
        &format!(
            "<h1>Not published</h1><div class=\"refusal\">{}</div>\
             <p class=\"lede\"><a href=\"/\">Everything published</a></p>",
            safe(why)
        ),
    )
}

#[cfg(test)]
mod tests;
