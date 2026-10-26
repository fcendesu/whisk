#!/usr/bin/env python3
"""Allowlisted local test diagnostics; raw SDK/server text is never retained."""
import argparse
import base64
import json
import os
import re
import sys

MESSAGES = {
    'starting RTC session': 'signaling_join',
    'peer connection state change': 'peer_connection',
    'ice connection state change': 'ice',
    'ice gathering state change': 'gathering',
    'closing participant': 'participant_close',
    'received offer': 'offer',
    'sending answer': 'answer',
    'received ICE candidate': 'remote_candidate',
    'added ICE candidate': 'remote_candidate',
    'sending ICE candidate': 'local_candidate',
}
STATES = {'new', 'checking', 'connecting', 'connected', 'completed',
          'disconnected', 'failed', 'closed', 'gathering', 'complete'}


def server_records(lines, roles):
    for line in lines:
        start = line.find('{')
        if start < 0:
            continue
        prefix = line[:start].strip().split('\t')
        if not re.fullmatch(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z', prefix[0]):
            continue
        try:
            data = json.loads(line[start:])
        except ValueError:
            continue
        if not isinstance(data, dict):
            continue
        event = MESSAGES.get(prefix[-1].strip())
        if event is None:
            continue
        identity = data.get('participant')
        role = roles.get(identity, 'unknown') if isinstance(identity, str) else 'unknown'
        if role not in {'publisher', 'viewer'}:
            role = 'unknown'
        record = {'time': prefix[0], 'event': event, 'role': role}
        if isinstance(data.get('state'), str) and data['state'] in STATES:
            record['state'] = data['state']
        if isinstance(data.get('target'), str) and data['target'] in {'PUBLISHER', 'SUBSCRIBER'}:
            record['target'] = data['target']
        yield record


def client_records(lines):
    for line in lines:
        match = re.search(r'Join \{ stage: (Publisher|Viewer), cause: (Connect \{ kind: (Timeout|Rejected|Transport) \}|InvalidInput) \}', line)
        if match:
            record = {'event': 'join_failure', 'stage': match[1].lower(),
                      'cause': 'connect' if match[3] else 'invalid_input'}
            if match[3]:
                record['kind'] = match[3].lower()
            yield record
        match = re.search(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;', line)
        if match:
            yield {'event': 'test_result', 'result': match[1].lower(),
                   'passed': int(match[2]), 'failed': int(match[3])}


def fixture_roles():
    roles = {}
    for name, role in [('LIVEKIT_PUBLISH_TOKEN', 'publisher'), ('LIVEKIT_TOKEN', 'viewer')]:
        try:
            part = os.environ[name].split('.')[1]
            claims = json.loads(base64.urlsafe_b64decode(part + '=' * (-len(part) % 4)))
            if isinstance(claims['sub'], str):
                roles[claims['sub']] = role
        except (KeyError, IndexError, ValueError, TypeError):
            continue
    return roles


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['server', 'client'])
    args = parser.parse_args()
    records = server_records(sys.stdin, fixture_roles()) if args.mode == 'server' else client_records(sys.stdin)
    for record in records:
        print(json.dumps(record))
