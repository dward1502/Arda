"""Exercise the real installer in isolated homes; never contact user systemd."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class UserUnitInstallTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.home = Path(self.tmp.name)
        self.units = self.home / '.config/systemd/user'
        self.units.mkdir(parents=True)
        self.hud = self.home / 'hud'
        self.hud.write_text('#!/bin/sh\nexit 0\n')
        self.hud.chmod(0o700)
        self.env = {**os.environ, 'HOME': str(self.home),
                    'ARDA_SYSTEMD_USER_DIR': str(self.units),
                    'ARDA_HUD_NATIVE_SOURCE': str(self.hud),
                    'ARDA_SKIP_SYSTEMD_RELOAD': 'true',
                    'XDG_RUNTIME_DIR': str(self.home)}
        self.dest = self.units / 'hermes-gateway.service.d/arda-watchdog.conf'

    def install(self):
        return subprocess.run(['bash', str(ROOT / 'scripts/install_arda_user_units.sh')],
                              env=self.env, capture_output=True, text=True)

    def test_installs_watchdog_without_overwriting_hermes_service(self):
        base = self.units / 'hermes-gateway.service'
        base.write_text('owned by Hermes\n')
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.dest.is_file(), 'Arda watchdog integration was not installed')
        self.assertEqual(self.dest.read_bytes(), (ROOT / 'config/systemd/hermes-gateway.service.d/arda-watchdog.conf').read_bytes())
        self.assertEqual(base.read_text(), 'owned by Hermes\n')

    def test_backup_failure_aborts_before_any_replacement(self):
        import shutil
        self.dest.parent.mkdir()
        self.dest.write_text('previous watchdog\n')
        session = self.units / 'arda-session.target'
        previous_session = (ROOT / 'config/systemd/arda-session.target').read_text() + '\n# previous session\n'
        session.write_text(previous_session)
        installed_hud = self.home / '.local/lib/arda/hud/arda_hud'
        installed_hud.parent.mkdir(parents=True)
        installed_hud.write_text('previous HUD\n')
        installed_hud.chmod(0o700)
        bindir = self.home / 'bin'
        bindir.mkdir()
        wrapper = bindir / 'install'
        wrapper.write_text(
            '#!/bin/bash\n'
            'if [[ "$3" == */arda-user-unit-install.*/arda-watchdog.conf ]]; then\n'
            '  exit 1\nfi\n'
            f'exec {shutil.which("install")} "$@"\n'
        )
        wrapper.chmod(0o700)
        self.env['PATH'] = str(bindir) + ':' + os.environ['PATH']
        result = self.install()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.dest.read_text(), 'previous watchdog\n')
        self.assertEqual(session.read_text(), previous_session)
        self.assertEqual(installed_hud.read_text(), 'previous HUD\n')

    def test_failed_rollback_retains_recovery_backup(self):
        import shutil
        self.dest.parent.mkdir()
        self.dest.write_text('previous watchdog\n')
        bindir = self.home / 'bin'
        bindir.mkdir()
        wrapper = bindir / 'install'
        wrapper.write_text(
            '#!/bin/bash\n'
            'if [[ "$2" == */arda-user-unit-install.*/arda-watchdog.conf ]]; then\n'
            '  exit 1\nfi\n'
            f'exec {shutil.which("install")} "$@"\n'
        )
        wrapper.chmod(0o700)
        systemctl = bindir / 'systemctl'
        systemctl.write_text('#!/bin/sh\nexit 1\n')
        systemctl.chmod(0o700)
        self.env.update(ARDA_SKIP_SYSTEMD_RELOAD='false',
                        PATH=str(bindir) + ':' + os.environ['PATH'],
                        TMPDIR=str(self.home))
        for key in ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS',
                    'XDG_SESSION_TYPE', 'XDG_RUNTIME_DIR'):
            self.env.pop(key, None)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        backups = list(self.units.glob('arda-user-unit-install.*/arda-watchdog.conf'))
        self.assertEqual(len(backups), 1, result.stderr)
        self.assertEqual(backups[0].read_text(), 'previous watchdog\n')
        self.assertIn(str(backups[0].parent), result.stderr)
        self.assertIn('rollback incomplete', result.stderr)

    def test_failed_reload_restores_previous_watchdog(self):
        self.dest.parent.mkdir()
        self.dest.write_text('previous watchdog\n')
        bindir = self.home / 'bin'
        bindir.mkdir()
        mock = bindir / 'systemctl'
        mock.write_text('#!/bin/sh\nexit 1\n')
        mock.chmod(0o700)
        self.env.update(ARDA_SKIP_SYSTEMD_RELOAD='false', PATH=str(bindir) + ':' + os.environ['PATH'])
        for key in ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS', 'XDG_SESSION_TYPE'):
            self.env.pop(key, None)
        # Avoid invoking a host session environment helper.
        self.env.pop('XDG_RUNTIME_DIR', None)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.dest.read_text(), 'previous watchdog\n')

    def test_failed_reload_removes_new_watchdog(self):
        bindir = self.home / 'bin'
        bindir.mkdir()
        mock = bindir / 'systemctl'
        mock.write_text('#!/bin/sh\nexit 1\n')
        mock.chmod(0o700)
        self.env.update(ARDA_SKIP_SYSTEMD_RELOAD='false', PATH=str(bindir) + ':' + os.environ['PATH'])
        for key in ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS', 'XDG_SESSION_TYPE', 'XDG_RUNTIME_DIR'):
            self.env.pop(key, None)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.dest.exists())


if __name__ == '__main__':
    unittest.main()
