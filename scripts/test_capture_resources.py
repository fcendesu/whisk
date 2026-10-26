import json,tempfile,unittest
from pathlib import Path
from sample_capture_resources import parse_sample,complete

class ResourceTests(unittest.TestCase):
    def test_only_attributable_numeric_fields_are_retained(self):
        self.assertEqual(parse_sample(' 123456 118.2\n'),{'rss_kib':123456,'ps_cpu_percent':118.2})
        self.assertIsNone(parse_sample('pid command env token'))
        self.assertIsNone(parse_sample('-1 12'))
    def test_local_preview_files_do_not_end_capture_sampler(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'measurement.json').write_text('{}')
            (root/'capture-stats.json').write_text('{}')
            (root/'ui-metrics.json').write_text(json.dumps({'mode':'local_capture'}))
            self.assertFalse(complete(root))
            (root/'ui-metrics.json').write_text(json.dumps({'mode':'captured_network'}))
            (root/'started.json').write_text('{}')
            (root/'capture-stats.json').write_text('{}')
            (root/'ui-metrics.json').write_text(json.dumps({'mode':'captured_network'}))
            (root/'outcome.json').write_text(json.dumps({'status':'completed'}))
            self.assertTrue(complete(root))

class OutcomeTests(unittest.TestCase):
    def test_failed_attempt_ends_sampler_without_a_success_measurement(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'started.json').write_text('{}')
            (root/'capture-stats.json').write_text('{}')
            (root/'ui-metrics.json').write_text(json.dumps({'mode':'captured_network'}))
            (root/'outcome.json').write_text(json.dumps({'status':'failed'}))
            self.assertTrue(complete(root))

    def test_idle_time_does_not_consume_the_observation_budget(self):
        from sample_capture_resources import budget_elapsed
        self.assertEqual(budget_elapsed(None,1000,0),0)
        self.assertEqual(budget_elapsed(1000,1002,0),2)
