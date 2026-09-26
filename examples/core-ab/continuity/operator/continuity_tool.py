"""Read-only MCP access to the isolated continuity treatment database."""
import json, os, subprocess, sys

if len(sys.argv) < 2:
    raise SystemExit('usage: python3 continuity_tool.py TOOL_NAME')
name = sys.argv[1]
allowed = {'aporic_resume', 'aporic_open', 'aporic_recall', 'aporic_task_list'}
if name not in allowed:
    raise SystemExit('read-only tool not allowed')
workspace = '/private/tmp/aporic-core-ab/continuity/treatment'
env = {**os.environ, 'APORIC_DATABASE':'/private/tmp/aporic-core-ab/continuity/treatment.sqlite3'}
p = subprocess.Popen(['/Users/wonyoung_choi/projects/aporic/target/release/aporic','mcp','serve','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1,env=env)
def exchange(request_id, method, params):
    p.stdin.write(json.dumps({'jsonrpc':'2.0','id':request_id,'method':method,'params':params})+'\n')
    p.stdin.flush()
    while True:
        line = p.stdout.readline()
        if not line:
            raise RuntimeError(p.stderr.read())
        reply = json.loads(line)
        if reply.get('id') == request_id:
            return reply
exchange(0,'initialize',{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'continuity-pilot','version':'1'}})
p.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');p.stdin.flush()
args={'workspace':workspace}
if name == 'aporic_open':
    args.update({'objective':'Continue interrupted shipping revision','idempotency_key':'continuity-treatment-open'})
reply=exchange(1,'tools/call',{'name':name,'arguments':args})
result=json.loads(reply['result']['content'][0]['text'])
print(json.dumps(result,ensure_ascii=False,indent=2))
p.stdin.close();p.terminate();p.wait(timeout=3)
