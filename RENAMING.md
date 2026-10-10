# Renaming the app

The app is currently called **Aventail 3D** (short: **Aventail**). The name is kept in as few places as possible so changing it is a short, mechanical job.

## What does NOT change on a rename
- **Crate and folder names** inside `crates/` are generic (`mesh`, `scene`, `uv`, `fileio`, `render`, `app`).
- **The project file format, `.rhy`.** It's tied to Rhylib, not to the app's name, so existing project files keep working.
- **Settings and saved layouts**, as long as `APP_ID` stays the same (see below).

## Checklist
1. **`crates/app/src/branding.rs`**: update `APP_NAME` and `APP_SHORT_NAME`. Everything in the app (window title, About box, menus, dialogs) reads from here.
2. **`APP_ID`** in the same file: leave it alone unless you really want to. It decides where settings are stored, so changing it makes the app start with fresh settings. If you do change it, add a one-time migration that copies the old settings folder.
3. **`crates/app/Cargo.toml`**: the `[[bin]] name` (the executable's file name) and the `description`.
4. **Installer and file association config**: the display name shown by the OS for the app and for `.rhy` files.
5. **Icons and splash screen** in `assets/`, if they contain the name or its initials.
6. **Docs**: search and replace the name in `README.md`, `DESIGN.md`, `CLAUDE.md`, `docs/manual/` and `docs/CHANGELOG.md`. Add a changelog line about the rename.
7. **GitHub**: rename the repository in its settings. GitHub redirects the old URL automatically, so existing links keep working.
8. **Decision log**: add a line to `DESIGN.md` §13.

Places get added to this list whenever the name appears somewhere new.
