import unittest
from gpu_metrics import extract_gpu_utilization

class GpuMetricsTests(unittest.TestCase):
    def test_only_valid_utilization_counters_are_retained(self):
        result = extract_gpu_utilization([{'PrivateSerial':'do-not-retain', 'PerformanceStatistics': {
            'Device Utilization %':35, 'Renderer Utilization %':False,
            'Tiler Utilization %':101, 'PrivateCounter':'secret'}}])
        self.assertEqual(result,[{'device_percent':35.0,'renderer_percent':None,'tiler_percent':None}])
        self.assertNotIn('secret',str(result))
        self.assertNotIn('do-not-retain',str(result))

    def test_missing_or_nonfinite_data_is_unknown(self):
        self.assertEqual(extract_gpu_utilization([]),[])
        result = extract_gpu_utilization([{'PerformanceStatistics':{'Device Utilization %':float('nan')}}])
        self.assertEqual(result,[{'device_percent':None,'renderer_percent':None,'tiler_percent':None}])

if __name__=='__main__': unittest.main()
