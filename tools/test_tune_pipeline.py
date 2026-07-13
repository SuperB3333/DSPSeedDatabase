import tempfile
import unittest
from pathlib import Path

from tune_pipeline import build_command, confirmed_updates, recommend, update_env_file


class TunePipelineTests(unittest.TestCase):
    def test_recommend_uses_smallest_candidate_within_threshold(self):
        recommended, peak = recommend(
            {1: [80.0, 81.0], 2: [96.0, 98.0], 4: [100.0, 99.0], 8: [90.0]},
            0.95,
        )

        self.assertEqual(2, recommended)
        self.assertEqual(4, peak)

    def test_docker_environment_placeholder_expands_to_arguments(self):
        command = build_command(
            "docker run --rm {docker_env} image",
            {"WORKER_THREADS": "8", "BENCHMARK": "1"},
        )

        self.assertEqual(
            [
                "docker",
                "run",
                "--rm",
                "-e",
                "BENCHMARK=1",
                "-e",
                "WORKER_THREADS=8",
                "image",
            ],
            command,
        )

    def test_env_update_preserves_unrelated_content(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "tuning.env"
            path.write_text("# retained\nWORKER_THREADS=4\nPG_PORT=5432\n")

            update_env_file(path, {"WORKER_THREADS": 8, "COMMIT_COUNT": 1000})

            self.assertEqual(
                "# retained\nWORKER_THREADS=8\nPG_PORT=5432\n\nCOMMIT_COUNT=1000\n",
                path.read_text(),
            )

    def test_rejecting_one_update_does_not_skip_later_updates(self):
        answers = iter(("no", "yes"))
        accepted = confirmed_updates(
            {"WORKER_THREADS": 8, "WRITER_THREADS": 4},
            Path("tuning.env"),
            lambda _: next(answers),
        )

        self.assertEqual({"WRITER_THREADS": 4}, accepted)


if __name__ == "__main__":
    unittest.main()
