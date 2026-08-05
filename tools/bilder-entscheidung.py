#!/usr/bin/env python3
"""Erzeugt die Bilder-Entscheidungsvorlage aus dem DATEIBESTAND.

    python3 tools/bilder-entscheidung.py     ->  docs/bilder-entscheidung/index.html
    Ansehen unter localhost:8000/rota-bilder/

**Von der Platte lesen, nicht aus dem Manifest.** Vier Painterly-Rahmen stehen
nicht in `docs/style-candidates/manifest.json` und haetten in der Vorlage
gefehlt — dieselbe Klasse wie „wer Zeilen zaehlt, vergleicht etwas anderes als
wer Werte zaehlt" (WORKER-GUIDE § 5).

Die Ausgabe ist in `.prettierignore`, weil sie erzeugt wird.
"""
import json, os, subprocess, html
from pathlib import Path

ROOT = Path('/work/rota')
OUT = ROOT / 'docs/bilder-entscheidung/index.html'
man = json.load(open(ROOT / 'docs/style-candidates/manifest.json'))

def groesse(p):
    try:
        b = os.path.getsize(ROOT/p)
        return f"{b//1024} KB" if b >= 1024 else f"{b} B"
    except OSError: return "?"

# --- A: ausgeliefert ---
icons = []
for autor in sorted(os.listdir(ROOT/'assets/img/icons')):
    d = ROOT/'assets/img/icons'/autor
    if not d.is_dir(): continue
    for f in sorted(os.listdir(d)):
        if f.endswith('.svg'):
            icons.append({'datei': f'assets/img/icons/{autor}/{f}', 'name': f[:-4], 'autor': autor})

# Schul-Zuordnung aus dem Manifest übernehmen, wo vorhanden
schule = {e['name']: e.get('school','—') for e in man['gameIcons']}
for i in icons: i['schule'] = schule.get(i['name'], '—')

frames = [{'datei':'assets/img/frames/slot-frame.png','name':'slot-frame',
           'zweck':'Rahmen um ein Zauber-Symbol auf der Leiste (104 px)'},
          {'datei':'assets/img/frames/ring-mount.png','name':'ring-mount',
           'zweck':'Fassung um das Reaktionsgefäß (den Boss-Ring)'}]
textur = [{'datei':'assets/img/textures/metal036-blackened-iron-512.jpg','name':'Metal036',
           'zweck':'Untergrund „geschwärztes Eisen" für Tafeln und Rahmen'}]

# **Von der Platte lesen, nicht aus dem Manifest** — sonst fehlt, was dort nicht steht.
# Gemessen: vier Rahmen-Dateien (frame-*.png) liegen im Ordner und im Manifest nicht.
_pm = {os.path.basename(e['file']): e for e in man['painterly']}
painterly = []
for f in sorted(os.listdir(ROOT/'docs/style-candidates/icons/painterly')):
    if not f.endswith(('.png','.jpg')): continue
    e = _pm.get(f)
    painterly.append(e if e else
        {'file': f'icons/painterly/{f}', 'name': f.rsplit('.',1)[0], 'school': '—', 'ohne_manifest': True})
kenney = man['kenney']
texturen_k = man['textures']

def karte(pfad_web, name, unterzeile, badge=None, id_=None):
    b = f'<span class="badge">{html.escape(badge)}</span>' if badge else ''
    return f'''<label class="karte" data-id="{html.escape(id_ or name)}">
  <div class="bild"><img loading="lazy" src="{pfad_web}" alt="{html.escape(name)}"></div>
  <div class="meta"><span class="name">{html.escape(name)}</span>{b}
    <span class="unter">{unterzeile}</span></div>
  <div class="wahl">
    <input type="radio" name="w-{html.escape(id_ or name)}" value="ja"><span class="ja">behalten</span>
    <input type="radio" name="w-{html.escape(id_ or name)}" value="nein"><span class="nein">raus</span>
  </div>
</label>'''

def web_assets(p): return '/rota-bilder/assets/' + p.split('assets/',1)[1]
def web_style(p):  return '/rota-bilder/style/' + p

teile = []

teile.append('<section id="a"><h2>A · Ausgeliefert &mdash; liegt in <code>assets/</code></h2>')
teile.append('''<p class="hinweis"><strong>Keines dieser Bilder erscheint heute im Spiel.</strong>
Gemessen: <code>grep -rn "img/icons" src/</code> findet nichts, und in den CSS-Dateien wird nur die
Textur geladen (in den Credits). Die 24 Symbole und die zwei Rahmen liegen im Repository und sind
nirgends verdrahtet &mdash; das ist ein eigener Befund, unabh&auml;ngig davon, wie du hier
entscheidest.</p>''')

teile.append('<h3>Die 24 Zauber-Symbole</h3>')
teile.append('''<p>Einfarbige SVG von <a href="https://game-icons.net">game-icons.net</a>,
<strong>CC BY 3.0</strong> &mdash; kommerziell nutzbar mit Namensnennung, die im Spiel unter CREDITS
steht. Sie sollen in der Schulfarbe eingef&auml;rbt werden (GAME-DESIGN 9.3). <strong>Nicht
KI-erzeugt.</strong> Es sind 24 f&uuml;r 15 gebaute Zauber &mdash; Version 1 braucht 60.</p>''')
for s in ['CALCINATION','FIXATION','SUBLIMATION','COAGULATION','PUTREFACTION','—']:
    grp = [i for i in icons if i['schule']==s]
    if not grp: continue
    teile.append(f'<h4 class="schule s-{s.lower()}">{s if s!="—" else "ohne Schul-Zuordnung"} <span class="anz">{len(grp)}</span></h4><div class="raster">')
    for i in grp:
        teile.append(karte(web_assets(i['datei']), i['name'],
                     f"{i['autor']} &middot; CC BY 3.0 &middot; {groesse(i['datei'])}",
                     id_='icon-'+i['name']))
    teile.append('</div>')

teile.append('<h3>Die zwei erzeugten Rahmen &mdash; das sind die KI-Bilder</h3>')
teile.append('''<p><strong>Eigenerzeugung mit <code>gpt-image</code>, 28.07.2026.</strong> Die
einzigen zwei Dateien im ganzen Repository, die eine Bildgenerierung erzeugt hat. Alles andere
stammt aus freien Sammlungen.</p><div class="raster gross">''')
for f in frames:
    teile.append(karte(web_assets(f['datei']), f['name'],
                 f"gpt-image, 28.07.2026 &middot; {groesse(f['datei'])}<br>{f['zweck']}",
                 badge='KI-erzeugt', id_='frame-'+f['name']))
teile.append('</div>')

teile.append('<h3>Die Textur</h3><div class="raster gross">')
for t in textur:
    teile.append(karte(web_assets(t['datei']), t['name'],
                 f"ambientCG &middot; CC0 &middot; {groesse(t['datei'])}<br>{t['zweck']}",
                 badge='im Spiel', id_='tex-'+t['name']))
teile.append('</div></section>')

teile.append('<section id="b"><h2>B &middot; Entscheidungsvorlage &mdash; liegt in <code>docs/style-candidates/</code></h2>')
teile.append('''<p class="hinweis">Diese Dateien wurden f&uuml;r die Stil-Entscheidung gesammelt
(Issue E9-1) und <strong>nie ausgeliefert</strong>. Sie stehen hier, weil du sie f&uuml;r deine
Entscheidung sehen sollst &mdash; nicht weil sie im Spiel w&auml;ren.</p>''')

teile.append(f'<h3>Painterly &mdash; die Alternative zu den einfarbigen Symbolen <span class="anz">{len(painterly)}</span></h3>')
teile.append('''<p>Gemalte PNG von <strong>J. W. Bjerk (eleazzaar)</strong>, OpenGameArt,
<strong>CC BY 3.0</strong>. Der Gegenentwurf: farbig und detailliert statt einfarbig und
einf&auml;rbbar. <strong>Wer sie w&auml;hlt, gibt die Einf&auml;rbung nach Schulfarbe auf</strong>
&mdash; die Farbe steckt dann im Bild und nicht in der Schule.</p><div class="raster">''')
for p in painterly:
    zusatz = ' &middot; <strong>nicht im Manifest</strong>' if p.get('ohne_manifest') else ''
    teile.append(karte(web_style(p['file']), p['name'],
                 f"J. W. Bjerk &middot; CC BY 3.0 &middot; {p.get('school','—')}{zusatz}",
                 badge='Rahmen' if p.get('ohne_manifest') else None,
                 id_='paint-'+p['name']))
teile.append('</div>')

teile.append(f'<h3>Kenney &mdash; Oberfl&auml;chen-Bausteine <span class="anz">{len(kenney)}</span></h3>')
teile.append('''<p><strong>CC0</strong>, keine Nennungspflicht. Kn&ouml;pfe und Rahmen f&uuml;r
Men&uuml;s. Sie sind der Gegenentwurf zu selbst gezeichneten Rahmen: fertig und stimmig, aber mit
eigenem Stil, der zum Rest passen muss.</p><div class="raster">''')
for k in kenney:
    nm = os.path.basename(k['file'])
    teile.append(karte(web_style(k['file']), nm, "Kenney &middot; CC0", id_='ken-'+nm))
teile.append('</div>')

teile.append(f'<h3>Texturen zur Auswahl <span class="anz">{len(texturen_k)}</span></h3>')
teile.append('<p><strong>CC0</strong> von ambientCG. Eine davon (Metal036) ist bereits in <code>assets/</code> &uuml;bernommen.</p><div class="raster gross">')
for t in texturen_k:
    nm = os.path.basename(t['file'])
    teile.append(karte(web_style(t['file']), nm, f"ambientCG &middot; CC0 &middot; {t.get('asset','')}", id_='tk-'+nm))
teile.append('</div>')

teile.append('''<h3>Verworfen</h3><p>Ein Kontaktbogen, der bei der Recherche ausgeschieden ist &mdash;
er steht hier nur der Vollst&auml;ndigkeit halber.</p><div class="raster gross">''')
teile.append(karte(web_style('rejected/pauliuw-cc0-kontaktbogen.png'), 'pauliuw-kontaktbogen',
             'bereits verworfen', badge='verworfen', id_='rej-pauliuw'))
teile.append('</div></section>')

gesamt = len(icons)+len(frames)+len(textur)+len(painterly)+len(kenney)+len(texturen_k)+1

kopf = f'''<!doctype html>
<html lang="de" data-theme="dark"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>ROTA &middot; Bilder zur Entscheidung</title>
<style>
:root {{
  --grund:#141110; --tafel:#1c1815; --linie:#2f2823; --text:#e8dfd2; --leise:#9a8c7c;
  --messing:#c9963f; --kupfer:#b4633a; --gruen:#4f8f6a; --rot:#a8503f;
}}
:root[data-theme="light"] {{
  --grund:#f2ece2; --tafel:#fbf7f0; --linie:#ddd2c2; --text:#241d17; --leise:#6b5d4e;
  --messing:#8a6520; --kupfer:#8d4a29; --gruen:#2f6a49; --rot:#8c3a2b;
}}
@media (prefers-color-scheme: light) {{ :root:not([data-theme]) {{
  --grund:#f2ece2; --tafel:#fbf7f0; --linie:#ddd2c2; --text:#241d17; --leise:#6b5d4e;
  --messing:#8a6520; --kupfer:#8d4a29; --gruen:#2f6a49; --rot:#8c3a2b; }} }}
* {{ box-sizing:border-box; }}
body {{ margin:0; background:var(--grund); color:var(--text);
  font:16px/1.6 ui-serif,'Iowan Old Style',Georgia,serif; }}
header {{ padding:3rem 2rem 2rem; border-bottom:1px solid var(--linie);
  background:linear-gradient(180deg,var(--tafel),transparent); }}
h1 {{ margin:0 0 .4rem; font-size:2rem; letter-spacing:.02em; font-weight:600; }}
h1 small {{ display:block; font-size:.9rem; color:var(--leise); letter-spacing:.16em;
  text-transform:uppercase; margin-bottom:.5rem; font-family:ui-monospace,monospace; }}
.wrap {{ max-width:1180px; margin:0 auto; padding:0 2rem 6rem; }}
section {{ margin-top:3.5rem; }}
h2 {{ font-size:1.35rem; border-bottom:2px solid var(--messing); padding-bottom:.4rem;
  display:inline-block; margin:0 0 1rem; }}
h3 {{ font-size:1.1rem; margin:2.2rem 0 .4rem; color:var(--messing); }}
h4.schule {{ font-family:ui-monospace,monospace; font-size:.78rem; letter-spacing:.18em;
  margin:1.6rem 0 .6rem; color:var(--leise); border-left:3px solid var(--kupfer); padding-left:.6rem; }}
.s-calcination {{ border-color:#c9702f; }} .s-fixation {{ border-color:#6f9fcc; }}
.s-sublimation {{ border-color:#bd9130; }} .s-coagulation {{ border-color:#35997e; }}
.s-putrefaction {{ border-color:#9b6fd0; }}
.anz {{ color:var(--leise); font-family:ui-monospace,monospace; font-size:.8rem; }}
p {{ max-width:62ch; color:var(--text); }}
p.hinweis {{ background:var(--tafel); border-left:3px solid var(--kupfer);
  padding:.9rem 1.1rem; border-radius:0 4px 4px 0; max-width:70ch; }}
code {{ font-family:ui-monospace,monospace; font-size:.86em; color:var(--messing); }}
a {{ color:var(--messing); }}
.raster {{ display:grid; gap:.9rem; grid-template-columns:repeat(auto-fill,minmax(158px,1fr)); margin:1rem 0 2rem; }}
.raster.gross {{ grid-template-columns:repeat(auto-fill,minmax(260px,1fr)); }}
.karte {{ background:var(--tafel); border:1px solid var(--linie); border-radius:6px;
  padding:.7rem; display:flex; flex-direction:column; gap:.55rem; cursor:pointer; }}
.karte:has(input[value=ja]:checked)   {{ border-color:var(--gruen); box-shadow:inset 0 0 0 1px var(--gruen); }}
.karte:has(input[value=nein]:checked) {{ border-color:var(--rot); opacity:.55; }}
.bild {{ aspect-ratio:1; display:grid; place-items:center;
  background:repeating-conic-gradient(#00000018 0% 25%, transparent 0% 50%) 50%/16px 16px; border-radius:4px; }}
.raster.gross .bild {{ aspect-ratio:16/10; }}
.bild img {{ max-width:100%; max-height:100%; }}
.meta {{ display:flex; flex-direction:column; gap:.15rem; }}
.name {{ font-family:ui-monospace,monospace; font-size:.82rem; }}
.unter {{ font-size:.72rem; color:var(--leise); line-height:1.4; }}
.badge {{ align-self:flex-start; font-family:ui-monospace,monospace; font-size:.62rem;
  letter-spacing:.1em; text-transform:uppercase; background:var(--kupfer); color:#fff;
  padding:.1rem .4rem; border-radius:3px; }}
.wahl {{ display:flex; gap:.4rem; font-size:.72rem; font-family:ui-monospace,monospace; }}
.wahl input {{ display:none; }}
.wahl span {{ flex:1; text-align:center; padding:.25rem; border:1px solid var(--linie);
  border-radius:3px; color:var(--leise); }}
input[value=ja]:checked + .ja    {{ background:var(--gruen); border-color:var(--gruen); color:#fff; }}
input[value=nein]:checked + .nein {{ background:var(--rot); border-color:var(--rot); color:#fff; }}
.leiste {{ position:sticky; bottom:0; background:var(--tafel); border-top:1px solid var(--messing);
  padding:.9rem 2rem; display:flex; gap:1rem; align-items:center; justify-content:space-between;
  font-family:ui-monospace,monospace; font-size:.82rem; }}
button {{ font:inherit; background:var(--messing); color:#1a1613; border:0; padding:.5rem 1rem;
  border-radius:4px; cursor:pointer; }}
#ausgabe {{ width:100%; min-height:9rem; margin-top:1rem; background:var(--grund); color:var(--text);
  border:1px solid var(--linie); border-radius:4px; padding:.8rem; font-family:ui-monospace,monospace;
  font-size:.76rem; display:none; }}
@media (prefers-reduced-motion:no-preference) {{ .karte {{ transition:border-color .15s, opacity .15s; }} }}
</style></head><body>
<header><div class="wrap" style="padding-bottom:0">
<h1><small>ROTA &middot; Stand {subprocess.run(['date','+%d.%m.%Y, %H:%M'],capture_output=True,text=True).stdout.strip()}</small>
Bilder zur Entscheidung</h1>
<p>Alle <strong>{gesamt}</strong> Bilddateien des Projekts, nach Zweck gegliedert. Zu jedem steht,
woher es stammt, welche Lizenz gilt und wof&uuml;r es gedacht ist. Entscheide pro Bild
<em>behalten</em> oder <em>raus</em> &mdash; am Ende erzeugt ein Knopf eine Liste, die du mir
zur&uuml;ckgeben kannst.</p>
<p class="hinweis"><strong>Zwei Dinge vorweg.</strong> Erstens: <strong>Nur zwei dieser Bilder sind
KI-erzeugt</strong> &mdash; die beiden Rahmen unter A. Alles andere stammt aus freien Sammlungen
(game-icons.net, OpenGameArt, Kenney, ambientCG) und ist CC0 oder CC&nbsp;BY. Zweitens:
<strong>Fast nichts davon erscheint heute im Spiel</strong>; geladen wird allein die Textur, und
zwar nur im Credits-Bereich.</p>
</div></header>
<div class="wrap">'''

fuss = '''</div>
<div class="leiste"><span id="zaehler">noch nichts entschieden</span>
<span><button id="knopf">Entscheidung als Liste</button></span></div>
<div class="wrap"><textarea id="ausgabe" readonly></textarea></div>
<script>
const karten = [...document.querySelectorAll('.karte')];
const zaehler = document.getElementById('zaehler');
function stand(){
  const ja = karten.filter(k=>k.querySelector('input[value=ja]').checked).length;
  const nein = karten.filter(k=>k.querySelector('input[value=nein]').checked).length;
  zaehler.textContent = `${ja} behalten \\u00b7 ${nein} raus \\u00b7 ${karten.length-ja-nein} offen`;
}
document.addEventListener('change', stand);
document.getElementById('knopf').addEventListener('click', () => {
  const zeilen = [];
  for (const k of karten) {
    const id = k.dataset.id;
    const ja = k.querySelector('input[value=ja]').checked;
    const nein = k.querySelector('input[value=nein]').checked;
    zeilen.push(`${ja ? 'BEHALTEN' : nein ? 'RAUS     ' : 'offen    '}  ${id}`);
  }
  const a = document.getElementById('ausgabe');
  a.style.display = 'block';
  a.value = zeilen.join('\\n');
  a.select();
});
stand();
</script></body></html>'''

OUT.write_text(kopf + '\n'.join(teile) + fuss, encoding='utf8')
print(f"geschrieben: {OUT}  ({OUT.stat().st_size//1024} KB, {gesamt} Bilder)")
