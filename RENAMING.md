# Renaming the app

The app is currently called **Aventail 3D** (short: **Aventail**). The name is kept in as few places as possible so changing it is a short, mechanical job.

## What does NOT change on a rename
- **Crate and folder names** inside `crates/` are generic (`mesh`, `uv`, `scene`, `config`, `fileio`, `render`, `tools`, `ui`, `app`).
- **The project file extension, `.avtl`.** It stays the same even if the app is renamed, so existing projects keep opening and double-clicking keeps working (many programs keep an old extension after a rename).
- **Settings and saved layouts**, as long as `APP_ID` stays the same (see below).

## Checklist
1. **`crates/app/src/branding.rs`**: update `APP_NAME` and `APP_SHORT_NAME`. Everything in the app (window title, About box, menus, dialogs) reads from here.
2. **`APP_ID`** in the same file: leave it alone unless you really want to. It decides where settings are stored, so changing it makes the app start with fresh settings. If you do change it, add a one-time migration that copies the old settings folder. The folder for backup copies (`autosave`, see [decision 0036](docs/decisions/0036-autosave.md)) sits in the same place and follows `APP_ID` too; the manual's Autosave page names it.
3. **`crates/app/Cargo.toml`**: the `[[bin]] name` (the executable's file name); the manual's Autosave page names it too (`aventail.exe`, in the Windows crash test).
4. **`.github/workflows/ci.yml`**: the `binary:` names in the Build job and the executable path in the Window test job, which must match the executable's file name; and the window title passed to `window-smoke.sh` in the Window test job, which must match `APP_NAME`.
5. **`Cargo.toml`** (root): the `repository` URL, if the GitHub repository is renamed.
6. **`NOTICE`**: the name and copyright line.
7. **Installer and file association config**: `installer/windows/setup.iss` and `installer/windows/build-setup.ps1` read `APP_NAME`, `APP_ID`, `PROJECT_EXTENSION`, `RELEASES_URL` and the executable's name from `branding.rs` and `crates/app/Cargo.toml`, so they need no change; the setup program's file name (`<APP_ID>-setup.exe`) and the release titles in `.github/workflows/release.yml` follow. If the GitHub repository is renamed, update `RELEASES_URL` and `RELEASES_API` in `branding.rs` (installed copies of older builds keep finding the releases through GitHub's redirect). Keep `APP_ID` (the installer's id): changing it makes Windows treat the new version as a different app, installed beside the old one. The manual's *Installing and updating* page names `aventail-setup.exe`.
8. **Icons and splash screen** in `assets/` (when they exist), if they contain the name or its initials.
9. **Docs**: search and replace the name in `README.md`, `DESIGN.md`, `CLAUDE.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `SECURITY.md`, `docs/manual/` and `.github/ISSUE_TEMPLATE/config.yml` (repository URL). Add a `CHANGELOG.md` entry about the rename.
10. **GitHub**: rename the repository in its settings. GitHub redirects the old URL automatically, so existing links keep working.
11. **Decision record**: add one to `docs/decisions/` explaining the new name.

Places get added to this list whenever the name appears somewhere new.
