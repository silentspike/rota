// Hook fuer die Suno-Seite: schneidet die ANTWORT des Generate-Calls mit,
// damit wir die Clip-UUIDs bekommen ohne den DOM zu parsen.
// Idempotent — mehrfaches Installieren ueberschreibt sauber.
() => {
  window.__gen = window.__gen || [];
  if (!window.__genOrigFetch) window.__genOrigFetch = window.fetch;
  window.fetch = async (...args) => {
    const res = await window.__genOrigFetch(...args);
    try {
      const u = args[0];
      const url = String(u && u.url ? u.url : u);
      if (url.includes('/api/generate/v2-web/')) {
        res.clone().json().then(d => {
          const clips = d.clips || (d.id ? [d] : []);
          window.__gen.push({
            t: Date.now(),
            tags: (clips[0] && clips[0].metadata && clips[0].metadata.tags) || null,
            ids: clips.map(c => c.id),
          });
        }).catch(() => {});
      }
    } catch (e) {}
    return res;
  };
  return 'gen-hook ready (' + window.__gen.length + ' erfasst)';
}
