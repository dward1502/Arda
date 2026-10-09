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

    def root_composition(self):
        import json
        release = self.home / 'relocated release'
        release.mkdir()
        (release / 'services.toml').write_text('[services]\n')
        config = self.home / 'runtime settings.env'
        config.write_text('# explicitly selected account settings\n')
        provider = self.home / 'providers.toml'
        provider.write_text('provider = []\n')
        composition = self.home / 'composition.json'
        composition.write_text(json.dumps(dict(
            release_root=str(release), root_executable=str(self.hud),
            operator_id='operator:qualification', environment_file=str(config),
            provider_config=str(provider), credentials={})))
        self.env['ARDA_ROOT_COMPOSITION'] = str(composition)
        return composition

    def test_explicit_root_composition_installs_relocated_identity(self):
        self.root_composition()
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        root = self.units / 'arda.service'
        self.assertTrue(root.is_file(), 'root composition omitted arda.service')
        text = root.read_text()
        self.assertIn(f'WorkingDirectory={self.home}/relocated release', text)
        self.assertIn('ARDA_OPERATOR_ID=operator:qualification', text)
        for unwanted in ('operator:mythos', '/Eregion/Arda', '.lo...tion',
                         '100.103.125.88', 'LoadCredential='):
            self.assertNotIn(unwanted, text)
        for name in ('arda-hud.service', 'arda-mirromere.service'):
            self.assertIn('ARDA_OPERATOR_ID=operator:qualification', (self.units / name).read_text())
        root.unlink()
        checked = subprocess.run(['bash', str(ROOT / 'scripts/verify_arda_user_units.sh'),
                                  str(self.units)], env=self.env, capture_output=True, text=True)
        self.assertNotEqual(checked.returncode, 0, 'root composition verifier accepted missing root')

    def test_invalid_root_composition_preserves_existing_units(self):
        import json
        composition = self.root_composition()
        data = json.loads(composition.read_text())
        data['environment_file'] = str(self.home / 'missing.env')
        composition.write_text(json.dumps(data))
        root = self.units / 'arda.service'
        root.write_text('previous root\n')
        result = self.install()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(root.read_text(), 'previous root\n')
        self.assertFalse((self.units / 'arda-session.target').exists())

    def test_failed_reload_restores_root_composition(self):
        self.root_composition()
        root = self.units / 'arda.service'
        root.write_text('previous root\n')
        bindir = self.home / 'bin'
        bindir.mkdir()
        command = bindir / 'systemctl'
        command.write_text('#!/bin/sh\n'
                           f'test -e "{self.home}/root-at-reload" || cp "{root}" "{self.home}/root-at-reload"\n'
                           'exit 1\n')
        command.chmod(0o700)
        self.env.update(ARDA_SKIP_SYSTEMD_RELOAD='false', PATH=str(bindir) + ':' + os.environ['PATH'])
        for key in ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS', 'XDG_SESSION_TYPE', 'XDG_RUNTIME_DIR'):
            self.env.pop(key, None)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(root.read_text(), 'previous root\n')
        self.assertFalse((self.units / 'arda-session.target').exists())
        self.assertIn('operator:qualification', (self.home / 'root-at-reload').read_text())

    def test_rendered_root_passes_native_systemd_parser(self):
        import shutil
        if not shutil.which('systemd-analyze'):
            self.skipTest('systemd-analyze not installed')
        import json
        composition = self.root_composition()
        secret = self.home / 'private capability'
        secret.write_text('synthetic-private-value')
        data = json.loads(composition.read_text())
        data['credentials'] = {'arda-manwe-mutation': str(secret)}
        composition.write_text(json.dumps(data))
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        checked = subprocess.run(['systemd-analyze', '--user', 'verify', '--man=no',
                                  str(self.units / 'arda.service')], env=self.env,
                                 capture_output=True, text=True)
        self.assertEqual(checked.returncode, 0, checked.stderr)
        self.assertEqual(checked.stderr.strip(), '', checked.stderr)

    def test_explicit_credentials_are_references_only(self):
        import json
        composition = self.root_composition()
        secret = self.home / 'capability'
        secret.write_text('synthetic-private-value')
        data = json.loads(composition.read_text())
        data['credentials'] = {'arda-manwe-mutation': str(secret)}
        composition.write_text(json.dumps(data))
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        text = (self.units / 'arda.service').read_text()
        self.assertIn(f'LoadCredential=arda-manwe-mutation:{secret}', text)
        self.assertNotIn('synthetic-private-value', text + result.stdout + result.stderr)
        self.assertEqual(secret.read_text(), 'synthetic-private-value')

    def test_composition_rejects_directive_injection_before_install(self):
        import json
        composition = self.root_composition()
        original = json.loads(composition.read_text())
        for key, value in [('operator_id', 'operator:other\nExecStart=/bin/false'),
                           ('release_root', str(self.home) + '/%h'),
                           ('credentials', {'bad\nname': str(self.hud)})]:
            composition.write_text(json.dumps({**original, key: value}))
            result = self.install()
            self.assertNotEqual(result.returncode, 0, key)
            self.assertFalse((self.units / 'arda.service').exists())
            self.assertFalse((self.units / 'arda-session.target').exists())

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
