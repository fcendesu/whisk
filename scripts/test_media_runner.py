import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent

class MediaRunnerTests(unittest.TestCase):
    def exercise_runner(self, diagnostic_failure=False):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            def command(name, text):
                path = folder / name
                path.write_text('#!/bin/sh\n'+text)
                path.chmod(0o755)
            command('docker', '''for arg in "$@"; do
case "$arg" in
port) echo 127.0.0.1:12345; exit 0;;
logs) echo logs >> "$WHISK_TEST_EVENTS"; echo '2026-10-26T07:00:00Z\tDEBUG\tx\tx\tice connection state change\t{"state":"checking","password":"private-password"}'; exit 0;;
down) echo down >> "$WHISK_TEST_EVENTS"; exit 0;;
esac
done
exit 0
''')
            command('curl', 'exit 0\n')
            command('cargo', '''if [ "${WHISK_TEST_DIAGNOSTIC_FAILURE:-0}" = 1 ]; then
  for run in "$WHISK_MEDIA_DIAGNOSTIC_DIR"/run-*; do mkdir "$run/client.jsonl"; done
fi
echo 'relay media failed: Join { stage: Viewer, cause: Connect { kind: Timeout } }'
echo 'test result: FAILED. 6 passed; 1 failed; 0 ignored;'
exit 23
''')
            diagnostics = folder / 'diagnostics'
            events = folder / 'events'
            env = dict(os.environ, PATH=str(folder)+os.pathsep+os.environ['PATH'],
                       WHISK_MEDIA_DIAGNOSTIC_DIR=str(diagnostics), WHISK_TEST_EVENTS=str(events), TMPDIR=str(folder),
                       WHISK_TEST_DIAGNOSTIC_FAILURE="1" if diagnostic_failure else "0")
            result = subprocess.run(['sh', 'scripts/test-native-media.sh'], cwd=ROOT, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 23, result.stdout+result.stderr)
            runs = list(diagnostics.glob('run-*'))
            self.assertEqual(len(runs), 1, 'diagnostic evidence was not retained')
            if diagnostic_failure:
                self.assertTrue((runs[0]/'client.jsonl').is_dir())
                self.assertIn('Unable to collect sanitized client diagnostics.', result.stderr)
            else:
                client = [json.loads(x) for x in (runs[0]/'client.jsonl').read_text().splitlines()]
                self.assertIn({'event':'join_failure','stage':'viewer','cause':'connect','kind':'timeout'},client)
                self.assertIn({'event':'runner_exit','exit_code':23},client)
            server = (runs[0]/'relay-server.jsonl').read_text()
            self.assertIn('checking', server)
            self.assertNotIn('private-password', server)
            self.assertEqual(events.read_text().splitlines(), ['logs','down','down'])
            self.assertEqual(list(folder.glob('whisk-native-test.*')), [])

    def test_failed_cargo_status_survives_diagnostic_collection_and_fixture_cleanup(self):
        self.exercise_runner()

    def test_diagnostic_write_failure_preserves_cargo_status_and_cleanup(self):
        self.exercise_runner(diagnostic_failure=True)
