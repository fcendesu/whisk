#!/usr/bin/env python3
"""Numeric process resources scoped to explicit Capture Lab experiments."""
import argparse,json,math,subprocess,time
from pathlib import Path

def parse_sample(output):
    fields=output.split()
    if len(fields)!=2:
        return None
    try:
        rss,cpu=map(float,fields)
    except ValueError:
        return None
    if not math.isfinite(rss) or not math.isfinite(cpu) or rss<0 or cpu<0:
        return None
    return {'rss_kib':int(rss),'ps_cpu_percent':cpu}

def complete(root):
    try:
        outcome=json.loads((root/'outcome.json').read_text())
        if outcome.get('status') not in ('completed','failed'):
            return False
        capture=root/'capture-stats.json'
        value=json.loads(capture.read_text())
        if value.get('capture_started') is False:
            return True
        start=(root/'started.json').stat().st_mtime
        if outcome.get('status')=='failed':
            return capture.stat().st_mtime>=start
        ui=root/'ui-metrics.json'
        return (json.loads(ui.read_text()).get('mode')=='captured_network'
                and capture.stat().st_mtime>=start and ui.stat().st_mtime>=start)
    except (OSError,ValueError):
        return False

def budget_elapsed(observation_start,now,launch):
    return 0 if observation_start is None else now-observation_start

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid',type=int,required=True)
    parser.add_argument('--result-dir',type=Path,required=True)
    parser.add_argument('--max-seconds',type=int,default=900)
    args=parser.parse_args()
    if args.pid<=0 or not 1<=args.max_seconds<=1800:
        parser.error('Positive PID and1–1800second observation budget required')
    launch=time.monotonic();observation=None
    while True:
        now=time.monotonic()
        if observation is None and (args.result_dir/'started.json').exists():
            observation=now
            print(json.dumps({'event':'observation_started','unix_ms':round(time.time()*1000)}),flush=True)
        if complete(args.result_dir):
            print(json.dumps({'event':'reports_saved','unix_ms':round(time.time()*1000)}),flush=True)
            return
        if budget_elapsed(observation,now,launch)>=args.max_seconds:
            print(json.dumps({'event':'observation_sampler_budget_expired'}),flush=True)
            return
        if observation is None and now-launch>=1800:
            print(json.dumps({'event':'idle_wait_budget_expired'}),flush=True)
            return
        result=subprocess.run(['ps','-p',str(args.pid),'-o','rss=,%cpu='],capture_output=True,text=True)
        sample=parse_sample(result.stdout) if result.returncode==0 else None
        if sample is None:
            print(json.dumps({'event':'process_unavailable'}),flush=True)
            return
        if observation is not None:
            print(json.dumps(dict(event='sample',unix_ms=round(time.time()*1000),observation_elapsed_ms=round((now-observation)*1000),**sample)),flush=True)
        time.sleep(1)

if __name__=='__main__':
    main()
