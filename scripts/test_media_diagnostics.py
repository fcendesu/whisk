import json
import unittest
from media_diagnostics import server_records, client_records

class MediaDiagnosticsTests(unittest.TestCase):
    def test_server_events_redact_sdp_addresses_credentials_and_identity(self):
        fields = {'participant': 'private-identity', 'state': 'checking', 'target': 'PUBLISHER',
                  'candidate': 'candidate:secret 1 udp 1 192.0.2.55 3478 typ relay',
                  'password': 'private-turn-password', 'sdp': 'private-sdp'}
        line = '2026-10-26T07:00:00Z\tDEBUG\tlivekit.transport\trtc/transport.go:1\tice connection state change\t' + json.dumps(fields)
        records = list(server_records([line], {'private-identity': 'publisher'}))
        self.assertEqual(records, [{'time': '2026-10-26T07:00:00Z', 'event': 'ice', 'role': 'publisher', 'state': 'checking', 'target': 'PUBLISHER'}])
        for private in ['private-', '192.0.2.55', 'candidate:']:
            self.assertNotIn(private, json.dumps(records))

    def test_unknown_server_messages_and_non_allowlisted_states_are_excluded(self):
        lines = ['private-token arbitrary text',
                 '2026-10-26T07:00:00Z\tDEBUG\tx\tx\tunsafe message\t{"state":"connected"}',
                 '2026-10-26T07:00:00Z\tDEBUG\tx\tx\tice connection state change\t{"participant":"private-identity","state":"private-password","target":"192.0.2.55"}']
        self.assertEqual(list(server_records(lines, {})), [{'time': '2026-10-26T07:00:00Z', 'event': 'ice', 'role': 'unknown'}])

    def test_client_join_errors_and_result_counts_are_retained_without_raw_lines(self):
        lines = ['private bearer-token 192.0.2.55',
                 'relay media failed: Join { stage: Viewer, cause: Connect { kind: Timeout } }',
                 'test result: FAILED. 6 passed; 1 failed; 0 ignored; 3 measured; 0 filtered out; finished in 1s',
                 'private SDP candidate']
        self.assertEqual(list(client_records(lines)), [
            {'event': 'join_failure', 'stage': 'viewer', 'cause': 'connect', 'kind': 'timeout'},
            {'event': 'test_result', 'result': 'failed', 'passed': 6, 'failed': 1}])

    def test_invalid_input_is_not_replaced_by_an_invented_transport_failure(self):
        self.assertEqual(list(client_records(['Join { stage: Publisher, cause: InvalidInput }'])),
                         [{'event': 'join_failure', 'stage': 'publisher', 'cause': 'invalid_input'}])

    def test_actual_server_candidate_receipt_drops_candidate_details(self):
        line = '2026-10-26T07:00:00Z\tDEBUG\tx\tx\tadded ICE candidate\t' + json.dumps({'participant': 'private-id', 'candidate': 'candidate:private 1 udp 1 192.0.2.55 30001 typ relay'})
        self.assertEqual(list(server_records([line], {'private-id': 'publisher'})),
                         [{'time': '2026-10-26T07:00:00Z', 'event': 'remote_candidate', 'role': 'publisher'}])
