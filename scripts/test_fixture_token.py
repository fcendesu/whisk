import base64
import importlib.util
import json
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("fixture_token", Path(__file__).resolve().parents[1] / "spikes/media/fixture/token.py")
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)

class PrivateRoomTokenTests(unittest.TestCase):
    def test_operator_room_is_used_for_both_private_room_grants(self):
        for publish in [False, True]:
            encoded = fixture.token(publish=publish, room="whisk-audio-private").split(".")[1]
            claims = json.loads(base64.urlsafe_b64decode(encoded + "=" * (-len(encoded) % 4)))
            self.assertEqual(claims["video"]["room"], "whisk-audio-private")
            self.assertEqual(claims["video"]["canPublish"], publish)
            self.assertLessEqual(claims["exp"] - claims["nbf"], 125)
    def test_empty_room_is_rejected(self):
        for room in ["", "  "]:
            with self.assertRaises(ValueError):
                fixture.token(room=room)
