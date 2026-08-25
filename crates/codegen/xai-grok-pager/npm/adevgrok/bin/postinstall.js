#!/usr/bin/env node
'use strict';

// ADEVGrok postinstall: downloads the prebuilt ADEVGrok binary for this
// platform from the GitHub Releases of Asif2902/grok-adev-support and
// installs it to ~/.adevgrok/bin as:
//
//   Unix:    adevgrok-<version>  +  adevgrok  (symlink)
//   Windows: adevgrok-<version>.exe + adevgrok.exe (copy)
//
// The npm tarball ships no binaries; everything is fetched at install time
// and checksum-verified against the `<asset>.sha256` published next to it.
//
// Design goals:
// - zero npm dependencies (Node >= 18 global fetch + crypto only)
// - never break `npm install`: unsupported platforms and network failures
//   exit 0 with instructions; `bin/adevgrok` retries on first launch
// - Termux/Android ARM64 detection -> native android-aarch64 asset

const fs = require('fs');
const os = require('os');
const path = require('path');
const crypto = require('crypto');

const PKG_NAME = 'adevgrok';
const DEFAULT_REPO = 'Asif2902/grok-adev-support';
const DOWNLOAD_TIMEOUT_MS = 10 * 60 * 1000;

function pkgVersion() {
    try { return require('../package.json').version; } catch { return null; }
}

function homeDir() {
    return process.env.ADEVGROK_HOME || path.join(os.homedir(), '.adevgrok');
}

function binDir() {
    return path.join(homeDir(), 'bin');
}

// --- platform / target detection -------------------------------------------

function isTermux() {
    if (process.platform === 'android') return true;
    const prefix = process.env.PREFIX || process.env.TERMUX_PREFIX || '';
    if (prefix.includes('com.termux')) return true;
    try {
        return fs.existsSync('/data/data/com.termux/files/usr');
    } catch {
        return false;
    }
}

// Returns the release-asset platform suffix, or null if we have no build.
// Mirrors .github/workflows/release.yml matrix.platform values.
function detectTarget() {
    const p = process.platform;
    const a = process.arch;
    if ((p === 'linux' || p === 'android') && a === 'arm64') {
        // Termux runs Bionic libc: needs the android build, NOT linux-aarch64.
        return isTermux() ? 'android-aarch64' : 'linux-aarch64';
    }
    if (p === 'linux' && a === 'x64') return 'linux-x86_64';
    if (p === 'darwin' && a === 'arm64') return 'macos-aarch64';
    if (p === 'win32' && a === 'x64') return 'windows-x86_64';
    return null;
}

function unsupportedMessage(target) {
    console.error(`${PKG_NAME}: no prebuilt binary for ${process.platform}-${process.arch}.`);
    console.error('');
    if (target === null && process.platform === 'darwin' && process.arch === 'x64') {
        console.error('macOS Intel (x86_64) is not built by the release pipeline.');
        console.error('Build from source instead:');
        console.error('  https://github.com/' + repoSlug() + '#building-from-source');
    } else {
        console.error(`Prebuilt targets are: android-aarch64, linux-x86_64, linux-aarch64, macos-aarch64, windows-x86_64.`);
        console.error('You can build from source:');
        console.error('  https://github.com/' + repoSlug() + '#building-from-source');
    }
}

// --- release asset resolution ------------------------------------------------

function repoSlug() {
    return process.env.ADEVGROK_GITHUB_REPO || DEFAULT_REPO;
}

function assetName(version, target) {
    return `adevgrok-${version}-${target}${process.platform === 'win32' ? '.exe' : ''}`;
}

async function fetchJson(url) {
    const res = await fetch(url, {
        headers: {
            'User-Agent': `${PKG_NAME}-installer`,
            'Accept': 'application/vnd.github+json',
        },
        signal: AbortSignal.timeout(30 * 1000),
    });
    if (!res.ok) throw new Error(`${url} -> HTTP ${res.status}`);
    return res.json();
}

// Resolve the download URL for the current target. Primary path: GitHub API
// "latest release" lookup (decoupled from the npm package version). Fallback:
// version-pinned URL derived from this package's own version.
async function resolveAssetUrl(target) {
    const slug = repoSlug();
    const suffix = process.platform === 'win32' ? '.exe' : '';
    const wanted = new RegExp(`^adevgrok-(.+)-${target.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}${suffix}$`);

    try {
        const release = await fetchJson(`https://api.github.com/repos/${slug}/releases/latest`);
        const assets = Array.isArray(release.assets) ? release.assets : [];
        const match = assets.find((a) => wanted.test(a.name));
        if (match && match.browser_download_url) {
            const m = match.name.match(wanted);
            return { url: match.browser_download_url, version: m[1] };
        }
        throw new Error(`latest release has no ${target} asset`);
    } catch (err) {
        const version = pkgVersion();
        if (!version) throw err;
        console.log(`${PKG_NAME}: API lookup failed (${err.message}); falling back to v${version} URL.`);
        return {
            url: `https://github.com/${slug}/releases/download/v${version}/${assetName(version, target)}`,
            version,
        };
    }
}

// --- download + verify -------------------------------------------------------

async function download(url) {
    const res = await fetch(url, { signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS) });
    if (!res.ok) throw new Error(`${url} -> HTTP ${res.status}`);
    return Buffer.from(await res.arrayBuffer());
}

async function sha256OfUrl(url) {
    try {
        const res = await fetch(url, {
            signal: AbortSignal.timeout(30 * 1000),
        });
        if (!res.ok) return null;
        const text = (await res.text()).trim();
        // sha256sum format: "<hex>  <filename>"
        const hex = text.split(/\s+/)[0];
        return /^[a-fA-F0-9]{64}$/.test(hex) ? hex.toLowerCase() : null;
    } catch {
        return null;
    }
}

async function downloadAndVerify(url) {
    const expected = await sha256OfUrl(`${url}.sha256`);
    const buf = await download(url);
    if (expected) {
        const actual = crypto.createHash('sha256').update(buf).digest('hex');
        if (actual !== expected) {
            throw new Error(
                `checksum mismatch: expected sha256 ${expected}, got ${actual}. ` +
                `Delete your npm cache and retry.`);
        }
        console.log(`${PKG_NAME}: sha256 verified OK`);
    } else {
        console.warn(`${PKG_NAME}: WARNING no .sha256 published for this asset; skipping verification.`);
    }
    return buf;
}

// --- install -----------------------------------------------------------------

function writeFileAtomic(filePath, buf) {
    const tmp = `${filePath}.tmp.${process.pid}`;
    fs.writeFileSync(tmp, buf);
    fs.renameSync(tmp, filePath);
}

function installBinary(buf, version) {
    const dir = binDir();
    fs.mkdirSync(dir, { recursive: true });
    const IS_WINDOWS = process.platform === 'win32';
    const EXE = IS_WINDOWS ? '.exe' : '';

    const versionedPath = path.join(dir, `${PKG_NAME}-${version}${EXE}`);
    if (!fs.existsSync(versionedPath)) {
        if (!buf || !buf.length) {
            throw new Error(`missing binary bytes for ${version}`);
        }
        writeFileAtomic(versionedPath, buf);
    }
    if (!IS_WINDOWS) {
        try { fs.chmodSync(versionedPath, 0o755); } catch {}
    }

    const canonicalPath = path.join(dir, `${PKG_NAME}${EXE}`);
    if (IS_WINDOWS) {
        // Symlinks need elevation on Windows; copy instead. If the exe is
        // locked by a running instance, move it aside then retry.
        const oldPath = canonicalPath + '.old';
        try { fs.unlinkSync(oldPath); } catch {}
        let copied = false;
        try {
            try { fs.unlinkSync(canonicalPath); } catch {}
            fs.copyFileSync(versionedPath, canonicalPath);
            copied = true;
        } catch {
            // Likely locked by a running process: stash the old exe and retry.
            try { fs.renameSync(canonicalPath, oldPath); } catch {}
            try {
                fs.copyFileSync(versionedPath, canonicalPath);
                copied = true;
            } catch {
                try { fs.renameSync(oldPath, canonicalPath); } catch {}
            }
        }
        if (!copied) {
            throw new Error(
                `could not update ${canonicalPath}; close running ${PKG_NAME} processes and retry`);
        }
    } else {
        const tmpLink = canonicalPath + `.link.${process.pid}`;
        try { fs.unlinkSync(tmpLink); } catch {}
        fs.symlinkSync(path.basename(versionedPath), tmpLink);
        fs.renameSync(tmpLink, canonicalPath);
    }

    return canonicalPath;
}

// Best-effort installer marker so tooling can tell how the binary arrived.
// Only creates ~/.adevgrok/config.toml when absent; never rewrites user config.
function writeInstallerMarker() {
    try {
        const configPath = path.join(homeDir(), 'config.toml');
        if (fs.existsSync(configPath)) return;
        fs.mkdirSync(homeDir(), { recursive: true });
        fs.writeFileSync(configPath, '[cli]\ninstaller = "npm"\n', 'utf8');
    } catch {}
}

// --- entry point -------------------------------------------------------------

async function install({ quiet = false } = {}) {
    const target = detectTarget();
    if (!target) {
        unsupportedMessage();
        return null;
    }

    const { url, version } = await resolveAssetUrl(target);
    const IS_WINDOWS = process.platform === 'win32';
    const EXE = IS_WINDOWS ? '.exe' : '';
    const versionedPath = path.join(binDir(), `${PKG_NAME}-${version}${EXE}`);

    // Skip the download only when this exact version is already on disk.
    // A stale `~/.adevgrok/bin/adevgrok` symlink/copy from an older release
    // must not block `npm update` from fetching the new GitHub asset.
    let buf = null;
    if (!fs.existsSync(versionedPath)) {
        if (!quiet) {
            console.log(`${PKG_NAME}: downloading ${target} binary (${version}) from GitHub Releases...`);
            console.log('(first launch may take a minute depending on connection)');
        }
        buf = await downloadAndVerify(url);
    } else if (!quiet) {
        console.log(`${PKG_NAME}: ${version} already present at ${versionedPath}`);
    }

    const installed = installBinary(buf, version);
    writeInstallerMarker();
    if (!quiet && buf) console.log(`${PKG_NAME} ${version} installed: ${installed}`);
    return installed;
}

module.exports = { install, detectTarget };

if (require.main === module) {
    (async () => {
        if (process.env.ADEVGROK_SKIP_DOWNLOAD === '1') {
            console.log('ADEVGROK_SKIP_DOWNLOAD=1 set; skipping binary download.');
            console.log(`The binary will be fetched automatically on first \`${PKG_NAME}\` run.`);
            return;
        }
        try {
            await install();
        } catch (err) {
            console.error(`${PKG_NAME}: download failed: ${err.message}`);
            console.error(`No problem: the binary will be downloaded on first \`${PKG_NAME}\` run.`);
        }
    })();
}
