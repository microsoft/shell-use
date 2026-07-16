import asyncio
import os
import sys
import tempfile
import unittest
from pathlib import Path

import shell_use
from shell_use import ExpectationError, ShellUse

BIN = os.environ.get("SHELL_USE_BIN")
SHELL = "pwsh" if sys.platform == "win32" else None


def run(coro):
    return asyncio.run(coro)


@unittest.skipUnless(BIN, "set SHELL_USE_BIN to the shell-use binary to run integration tests")
class IntegrationTests(unittest.TestCase):
    def setUp(self):
        self._home = tempfile.mkdtemp(prefix="shell-use-py-")
        self._session = f"pytest-{os.getpid()}"

    def _client(self):
        return ShellUse(self._session, home=self._home)

    def test_echo_roundtrip(self):
        async def scenario():
            async with self._client() as su:
                await su.open(shell=SHELL)
                await su.submit("echo hello-sdk")
                await su.wait_command()
                await su.expect_text("hello-sdk", strict=False)
                await su.expect_exit_code(0)
                st = await su.state()
                self.assertGreater(st.cols, 0)

        run(scenario())

    def test_expect_text_error_includes_terminal(self):
        async def scenario():
            async with self._client() as su:
                await su.run(
                    sys.executable,
                    "-c",
                    "import sys,time; sys.stdout.write('ready'); "
                    "sys.stdout.flush(); time.sleep(60)",
                )
                await su.wait_text("ready", timeout=2000)
                with self.assertRaises(ExpectationError) as raised:
                    await su.expect_text("text-that-is-not-on-screen", timeout=50)
                message = str(raised.exception)
                self.assertIn(
                    "expect_text: timed out after 50ms waiting for "
                    "'text-that-is-not-on-screen' to be visible",
                    message,
                )
                self.assertIn("Terminal content:\n╭", message)
                self.assertIn("ready", message)
                self.assertIn("\n╰", message)

        run(scenario())

    def test_sessions_lists_open_session(self):
        async def scenario():
            su = self._client()
            await su.open(shell=SHELL)
            try:
                names = await shell_use.sessions(home=self._home)
                self.assertIn(self._session, names)
            finally:
                await su.close()

        run(scenario())

    def test_snapshot_lands_in_client_cwd(self):
        async def scenario():
            original = os.getcwd()
            snap_root = tempfile.mkdtemp(prefix="shell-use-snap-")
            name = f"snap-{os.path.basename(snap_root)}"
            async with self._client() as su:
                await su.open(shell=SHELL)
                await su.submit("echo snapshot-marker")
                await su.wait_command()
                os.chdir(snap_root)
                try:
                    status = await su.expect_snapshot(name)
                    self.assertEqual(status, "written")
                    created = Path(snap_root) / "__snapshots__" / f"{name}.snap"
                    self.assertTrue(created.is_file())
                    daemon_side = Path(original) / "__snapshots__" / f"{name}.snap"
                    self.assertFalse(daemon_side.exists())
                    self.assertEqual(await su.expect_snapshot(name), "passed")
                finally:
                    os.chdir(original)

        run(scenario())


if __name__ == "__main__":
    unittest.main()
