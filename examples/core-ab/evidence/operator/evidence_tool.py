"""MCP bridge for the isolated evidence-treatment pilot database."""
import json, os, subprocess, sys
if len(sys.argv)<2: raise SystemExit('usage: evidence_tool.py TOOL [JSON_ARGS_FILE]')
name=sys.argv[1]
allowed={'aporic_open','aporic_check_register','aporic_run_list','aporic_run_get','aporic_claim_assert','aporic_close'}
if name not in allowed: raise SystemExit('tool not allowed')
workspace='/private/tmp/aporic-core-ab/evidence/treatment'
args=json.load(open(sys.argv[2])) if len(sys.argv)>2 else {}
if name=='aporic_open': args={'workspace':workspace,'objective':'Repair the NDJSON event reader and calibrate completion claim','idempotency_key':'evidence-treatment-open'}
if name=='aporic_run_list': args.setdefault('workspace',workspace)
p=subprocess.Popen(['/Users/wonyoung_choi/projects/aporic/target/release/aporic','mcp','serve','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1,env={**os.environ,'APORIC_DATABASE':'/private/tmp/aporic-core-ab/evidence/treatment.sqlite3'})
i=0
def call(method,params):
 global i
 i+=1
 p.stdin.write(json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params})+'\n');p.stdin.flush()
 while True:
  line=p.stdout.readline()
  if not line: raise RuntimeError(p.stderr.read())
  r=json.loads(line)
  if r.get('id')==i:return r
call('initialize',{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'evidence-pilot','version':'1'}})
p.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');p.stdin.flush()
r=call('tools/call',{'name':name,'arguments':args})
print(json.dumps(json.loads(r['result']['content'][0]['text']),ensure_ascii=False,indent=2))
p.stdin.close();p.terminate();p.wait(timeout=3)
