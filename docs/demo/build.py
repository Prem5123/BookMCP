#!/usr/bin/env python3
"""Build a seekable presentation from verified captured output, without network calls."""
import html
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
capture = json.loads((ROOT / "capture.json").read_text())
cli = capture["cli"]
lesson = json.loads(cli[4]["stdout"])
retrieved = json.loads(capture["mcp"][-1]["response"]["result"]["content"][0]["text"])["lessons"][0]
assert lesson == retrieved
assert lesson["citation"]["page_start"] == 2
excerpt = " ".join(cli[3]["stdout"].split("A retry", 1)[1].split("Store the key", 1)[0].split())
excerpt = "A retry " + excerpt


def pre(text, cls="output"):
    return f'<pre class="{cls}">{html.escape(text)}</pre>'


def command(index, wrapped=None):
    return pre("$ " + (wrapped or " ".join(cli[index]["command"])), "command")


save_command = ('$ bookmcp lesson add reliable-code reliable-code-000002 \\\n'
                '  --title "Safe retries" \\\n'
                '  --body "Use an idempotency key before retrying side effects." \\\n'
                '  --json')
beats = [
    (0, 5, "Bring your books into system design.", "KNOWLEDGE FOR YOUR AGENT", "",
     '<div class="hero"><div class="hero-line">book evidence <span>→</span> design decisions</div>'
     '<div class="hero-line">saved lessons <span>→</span> implementation</div>'
     '<p>Real CLI + MCP. All book content stays local.</p></div>'),
    (5, 8, "Add the books behind your architecture.", "01 / INGEST · CLI", command(1),
     pre(cli[1]["stdout"].strip()) + '<div class="evidence">3 pages · 3 chapters · 3 searchable chunks</div>'
     '<p class="detail">Original programming guide included in this repository.</p>'),
    (13, 11, "Ground the design in a cited principle.", "02 / RETRIEVE · CLI + MCP",
     command(2) + command(3),
     f'<blockquote>{html.escape(excerpt)}</blockquote>'
     '<div class="citation">A Small Guide to Reliable Code · p. 2</div>'
     '<p class="detail">Chunk reliable-code-000002 · also retrieved with book_get_chunk</p>'),
    (24, 10, "Keep the principle behind the implementation.", "03 / SAVE · CLI", pre(save_command, "command"),
     pre(json.dumps({key: lesson[key] for key in ["lesson_id", "title", "stale"]}, indent=2)[2:-2])
     + '<div class="citation">Source citation and PDF hash saved automatically.</div>'),
    (34, 11, "Carry design knowledge into implementation.", "04 / RECALL · READ-ONLY MCP",
     pre('tools/call → book_list_lessons\n{"book_id": "reliable-code"}', "command"),
     pre(json.dumps({key: retrieved[key] for key in ["lesson_id", "title", "body", "stale"]}, indent=2))
     + '<div class="citation">Cited to page 2 · retrieved from a freshly started server</div>'),
]
panels, titles, labels = [], [], []
for i, (start, duration, title, label, cmd, output) in enumerate(beats):
    timing = f'class="clip" data-start="{start}" data-duration="{duration}"'
    titles.append(f'<div id="title-clip-{i}" {timing} data-track-index="{2+i*3}"><h1 id="title-{i}">{title}</h1></div>')
    labels.append(f'<div id="label-clip-{i}" {timing} data-track-index="{3+i*3}"><span class="tab">{label}</span></div>')
    panels.append(f'<section id="panel-clip-{i}" {timing} data-track-index="{4+i*3}"><div class="panel" id="panel-{i}">'
                  f'{cmd}<div class="result" id="result-{i}">{output}</div></div></section>')

page = '''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=1280, height=720">
<script src="node_modules/gsap/dist/gsap.min.js"></script>
<style>
*{box-sizing:border-box;margin:0} html,body,#root{width:1280px;height:720px;overflow:hidden}
body{font-family:"Montserrat",sans-serif;color:#182a25} #root{position:relative}
.paper{position:absolute;inset:0;background:#f2eee4}
.masthead{position:absolute;left:54px;right:54px;top:29px;display:flex;align-items:center;justify-content:space-between}
.brand{font-weight:900;font-size:30px;letter-spacing:-1px}.category{font-family:"IBM Plex Mono",monospace;font-size:16px;font-weight:700}
.headlines{position:absolute;left:54px;right:54px;top:89px;height:116px}
h1{font-size:43px;line-height:1.14;letter-spacing:-1.5px;max-width:1130px;font-weight:700}
.terminal{position:absolute;left:54px;right:54px;top:200px;height:418px;border-radius:13px;overflow:hidden;background:#182a25;color:#f2eee4;box-shadow:0 14px 30px #182a2522}
.chrome{height:49px;border-bottom:1px solid #52645d;display:flex;align-items:center;padding:0 24px;gap:10px}
.dot{width:10px;height:10px;border-radius:50%;background:#98d9ac}.chrome-text{font-family:"IBM Plex Mono",monospace;font-size:14px;margin-left:auto;color:#d3dfd6}
.tabs{position:absolute;top:16px;left:98px;width:700px;height:24px}.tab{font-family:"IBM Plex Mono",monospace;font-size:14px;font-weight:700;color:#d3dfd6}
.panel{position:absolute;top:49px;left:0;width:1172px;height:369px;padding:23px 29px;font-family:"IBM Plex Mono",monospace}
pre{white-space:pre-wrap;overflow-wrap:anywhere;max-width:1110px;line-height:1.55}
.command{font-size:20px;color:#98d9ac;margin-bottom:14px}.output{font-size:21px;color:#f2eee4}.result{margin-top:7px}
.evidence{font-size:24px;color:#98d9ac;margin-top:36px;font-weight:700}.detail{font-size:17px;color:#d3dfd6;margin-top:18px}
blockquote{font-family:"Montserrat",sans-serif;font-size:27px;line-height:1.4;font-weight:700;max-width:1040px;margin-top:20px}
.citation{font-size:18px;color:#98d9ac;margin-top:15px}.hero{padding:17px 0}.hero-line{font-family:"Montserrat",sans-serif;font-size:44px;font-weight:700;line-height:1.6;letter-spacing:-1px}.hero-line span{color:#98d9ac}.hero p{margin-top:19px;font-size:20px;color:#d3dfd6}
.footer{position:absolute;left:54px;right:54px;top:641px;display:flex;justify-content:space-between;font-family:"IBM Plex Mono",monospace;font-size:14px;font-weight:700}
.track{position:absolute;bottom:26px;left:54px;right:54px;height:5px;background:#d5d9cf}.fill{width:100%;height:100%;background:#326d49;transform-origin:left center}
.clip{position:absolute;inset:0}
</style></head><body>
<div id="root" data-composition-id="main" data-start="0" data-duration="45" data-width="1280" data-height="720">
<div class="paper"></div>
<header class="masthead"><div class="brand">BookMCP</div><div class="category">BOOK KNOWLEDGE FOR SYSTEM DESIGN</div></header>
<div class="headlines">TITLES</div>
<div class="terminal"><div class="chrome"><i class="dot"></i><i class="dot"></i><i class="dot"></i><span class="chrome-text">bookmcp 0.1.0 · local</span></div><div class="tabs">LABELS</div>PANELS</div>
<footer class="footer"><span>github.com/Prem5123/BookMCP</span><span>Recorded output · excerpts · pauses shortened</span></footer>
<div class="track"><div class="fill" id="progress"></div></div>
</div><script>
window.__timelines=window.__timelines||{};
const tl=gsap.timeline({paused:true});
const beats=BEATS;
beats.forEach((start,i)=>{
 tl.fromTo(`#title-${i}`,{opacity:0,y:9},{opacity:1,y:0,duration:.35,ease:'power2.out',immediateRender:false},start+.05);
 tl.fromTo(`#panel-${i}`,{opacity:0},{opacity:1,duration:.25,immediateRender:false},start+.1);
 tl.fromTo(`#result-${i}`,{opacity:0,y:7},{opacity:1,y:0,duration:.3,ease:'power2.out',immediateRender:false},start+(i===0?.3:1.1));
});
tl.fromTo('#progress',{scaleX:0},{scaleX:1,duration:45,ease:'none'},0);
window.__timelines.main=tl;
</script></body></html>'''
page = page.replace("TITLES", "".join(titles)).replace("LABELS", "".join(labels))
page = page.replace("PANELS", "".join(panels)).replace("BEATS", json.dumps([b[0] for b in beats]))
(ROOT / "index.html").write_text(page)
print("Built 45-second composition from capture.json.")
