import json, os, subprocess
from pathlib import Path

binary = '/Users/wonyoung_choi/projects/aporic/target/release/aporic'
db = '/private/tmp/aporic-core-ab/continuity/treatment.sqlite3'
workspace = '/private/tmp/aporic-core-ab/continuity/treatment'
env = {**os.environ, 'APORIC_DATABASE': db}
p = subprocess.Popen([binary, 'mcp', 'serve', '--stdio'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1, env=env)
seq = 0

def call(name, args):
    global seq
    seq += 1
    request = {'jsonrpc':'2.0','id':seq,'method':'tools/call','params':{'name':name,'arguments':args}}
    p.stdin.write(json.dumps(request) + '\n')
    p.stdin.flush()
    while True:
        line = p.stdout.readline()
        if not line:
            raise RuntimeError(p.stderr.read())
        msg = json.loads(line)
        if msg.get('id') == seq:
            result = json.loads(msg['result']['content'][0]['text'])
            if not result.get('ok'):
                raise RuntimeError(f'{name}: {result}')
            return result['result']

p.stdin.write(json.dumps({'jsonrpc':'2.0','id':0,'method':'initialize','params':{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'core-ab-seed','version':'1'}}})+'\n')
p.stdin.flush()
while True:
    if json.loads(p.stdout.readline()).get('id') == 0:
        break
p.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n')
p.stdin.flush()
opened = call('aporic_open', {'workspace':workspace,'objective':'Continue the interrupted shipping revision','idempotency_key':'continuity-seed-open'})
sid = opened['session_id']

def record(kind, content, key, supersedes=None):
    result = call('aporic_record', {'session_id':sid,'kind':kind,'content':content,'evidence':None,'supersedes_record_id':supersedes,'verifies_effect_id':None,'idempotency_key':key})
    return result['record']['record_id']

old = record('decision', 'Initial draft: free shipping at 5,000 cents.', 'old-threshold')
record('decision', 'Current decision superseding the draft: free shipping at 6,000 cents, using subtotal before shipping.', 'new-threshold', old)
record('decision', 'Current rates below threshold: KR 400 cents; FR, DE, ES 700 cents; all other country codes 900 cents.', 'rates')
record('decision', 'Current loyalty rule: subtract 200 cents after choosing the regional rate, floor at zero; free shipping remains zero.', 'loyalty')
record('constraint', 'Negative subtotals raise ValueError; all money uses integer cents.', 'negative')
record('observation', 'Unrelated future design note: checkout banner color should eventually be navy.', 'banner')
task = call('aporic_task_create', {'session_id':sid,'objective':'Replace fixed shipping charge in shipping.py according to current decisions, add focused checks, and report verification','acceptance_criteria':['Current region rates and 6,000-cent free threshold are implemented','Loyalty discount and negative subtotal behavior are implemented','Focused checks pass and verification is reported'],'write_scope':['shipping.py','test_shipping_visible.py'],'depends_on':[],'idempotency_key':'shipping-task'})
call('aporic_close', {'session_id':sid,'disposition':'handoff','next_action':'Continue the queued shipping revision using current decisions and verify the changed function.','summary':'Revision was interrupted before code change. Current regional rates, 6,000-cent threshold, loyalty rule, and negative-subtotal rule were recorded.','idempotency_key':'seed-handoff'})
print(json.dumps({'db':db,'workspace':workspace,'session_id':sid,'task_id':task['task']['task_id']}, indent=2))
p.stdin.close();p.terminate();p.wait(timeout=3)
