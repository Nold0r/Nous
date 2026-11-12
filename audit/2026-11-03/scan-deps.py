"""Query public package names/versions only; never sends source or user data."""
import concurrent.futures
import json
from pathlib import Path
import sys
import tomllib
import urllib.request

root = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2]).resolve()
lock = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))
packages = [p for p in lock['package'] if p.get('source', '').startswith('registry+')]
queries = [{'package': {'name': p['name'], 'ecosystem': 'crates.io'}, 'version': p['version']} for p in packages]
request = urllib.request.Request('https://api.osv.dev/v1/querybatch',
    data=json.dumps({'queries': queries}).encode(), headers={'Content-Type': 'application/json'})
with urllib.request.urlopen(request, timeout=45) as response:
    results = json.load(response)['results']
hits = [{'package': p['name'], 'version': p['version'], 'vulns': r['vulns']}
        for p, r in zip(packages, results) if r.get('vulns')]
ids = sorted({v['id'] for h in hits for v in h['vulns']})
def fetch(identifier):
    with urllib.request.urlopen('https://api.osv.dev/v1/vulns/' + identifier, timeout=20) as response:
        return identifier, json.load(response)
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:
    records = dict(pool.map(fetch, ids))
out.write_text(json.dumps({'checked': len(packages), 'hits': hits, 'advisories': records},
    ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'Checked {len(packages)} registry packages; {len(hits)} package versions have advisories (including informational).')
for hit in hits:
    print(hit['package'], hit['version'], ', '.join(v['id'] for v in hit['vulns']))
