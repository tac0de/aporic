import json, os, subprocess
from pathlib import Path
binary='/Users/wonyoung_choi/projects/aporic/target/release/aporic'
db='/private/tmp/aporic-core-ab/context/treatment.sqlite3'
workspace='/private/tmp/aporic-core-ab/context/treatment'
records=[json.loads(line) for line in Path('/private/tmp/aporic-core-ab/context-fixture/treatment/import.jsonl').read_text().splitlines()]
p=subprocess.Popen([binary,'mcp','serve','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1,env={**os.environ,'APORIC_DATABASE':db})
i=0
def send(method,params):
 global i
 i+=1
 p.stdin.write(json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params})+'\n');p.stdin.flush()
 while True:
  line=p.stdout.readline()
  if not line: raise RuntimeError(p.stderr.read())
  r=json.loads(line)
  if r.get('id')==i: return r
send('initialize',{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'context-seed','version':'1'}})
p.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');p.stdin.flush()
def tool(name,args):
 r=send('tools/call',{'name':name,'arguments':args})
 v=json.loads(r['result']['content'][0]['text'])
 if not v.get('ok'): raise RuntimeError((name,v))
 return v['result']
sid=tool('aporic_open',{'workspace':workspace,'objective':'Preserve settlement rule revisions for a later implementation task','idempotency_key':'context-seed-open'})['session_id']
ids={}
reverse={r['superseded_by']:r['id'] for r in records if r['superseded_by']}
for record in records:
 content=f"{record['id']} | {record['date']} | {record['status']} | {record['topic']}: {record['text']}"
 prior=ids.get(reverse.get(record['id']))
 result=tool('aporic_record',{'session_id':sid,'kind':'observation' if record['id'].startswith('UI-') else 'decision','content':content,'evidence':None,'supersedes_record_id':prior,'verifies_effect_id':None,'idempotency_key':'context-'+record['id']})
 ids[record['id']]=result['record']['record_id']
tool('aporic_close',{'session_id':sid,'disposition':'completed','summary':'Six historical settlement and UI notes recorded; two settlement decisions supersede their older versions.','next_action':None,'idempotency_key':'context-seed-close'})
print(json.dumps({'db':db,'workspace':workspace,'record_ids':ids},indent=2))
p.stdin.close();p.terminate();p.wait(timeout=3)
