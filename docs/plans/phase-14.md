# Phase 14 plan: scripting

The working plan for Phase 14 ([decision 0299](../decisions/0299-scripting-plan.md); the roadmap is in [DESIGN.md](../../DESIGN.md#phase-14--scripting), the design in [DESIGN.md §4.7](../../DESIGN.md#47-scripting-phase-14)). It records what was chosen, how scripting is built, the controls, and a plan for every milestone. Update it as milestones are built.

**Written on 2026-11-17 while the project lead was away.** The project lead asked for the scripting milestones to be added but couldn't answer questions that day, so every choice below was made by Claude, taking the option most common in 3D programs, safest, and free of clashes with the app's keys. Each one is listed under [Choices to review](#choices-to-review) with its alternatives and what changing it later would cost. Milestones 1 to 3 were built the same day and milestones 4 to 6 the next, following them as written (choices 18 to 22, 23 to 29, 30 to 37, 38 to 46, 47 to 57 and 58 to 68 were added while building them); the project lead reviews them all later, and any answer that differs is changed before it's built on further.

## Choices to review

Each item: the question in plain words, what was chosen, the other ways, and what it would cost to change later.

1. **Which language do scripts use, and how do they run?**
   - **Chosen:** **LustScript** (`.lsc` files) is the default. The app holds the Lust compiler inside it, as Solis does: it checks a script, turns it into Lua, and runs that Lua in **Luau** (the small, safe Lua that Roblox made) through the `mlua` library. Nothing needs installing.
   - **Other ways:** (a) turn LustScript into Rust and build it: every user would need Rust and several gigabytes of build tools, and a build takes seconds or minutes; (b) run ordinary Lua (5.4 or LuaJIT) instead of Luau: no built-in way to stop a script reaching files and the network; (c) build plain Lua first and LustScript later.
   - **Cost of changing later:** high once scripts are shared, since scripts are written for the language; low before milestone 1 is built.
2. **How does plain Lua work?**
   - **Chosen:** both ways. Lua can be written inside a LustScript file in a `lua { }` block (as Lust already allows), **and** a whole script can be a plain `.lua` file. Both run in the same Luau with the same calls and the same safety.
   - **Other ways:** only `lua { }` blocks inside LustScript files (the literal reading of decision 0119); only whole `.lua` files.
   - **Cost of changing later:** small. Dropping `.lua` files later would break scripts people wrote that way.
3. **How does the app get the Lust compiler?**
   - **Chosen:** straight from Lust's GitHub repository, pinned to a release tag (`v2.0.0-alpha.73` today, the newest when milestone 1 starts), with `deny.toml` allowing that one repository, exactly as Solis does. Moving to a newer Lust is a pull request of its own that reads Lust's ENGINES.md.
   - **Other ways:** wait until Lust is published on crates.io; copy Lust's source into this repository.
   - **Cost of changing later:** tiny: one line in `Cargo.toml` and one in `deny.toml`.
4. **Can you keep working while a script runs?**
   - **Chosen:** a script works on a **copy** of the scene on another thread, so the app never freezes. While it runs you can still look around the view and read panels, but changes to that project wait (the status bar says a script is running, with a **Stop** button; other projects' tabs are free). When it finishes, everything it did lands at once as **one undo step**, named after the script.
   - **Other ways:** (a) run scripts on the interface thread in short slices (simpler, but long scripts stutter the app); (b) let you keep editing and merge the script's changes with yours afterwards (hard to make reliable: the two can clash).
   - **Cost of changing later:** medium: the running part of the `script` crate changes, scripts themselves don't.
5. **What may scripts reach, and how long may they run?**
   - **Chosen:** by default **nothing outside the project**: no files, no network, no other programs. A script can ask for one of three permissions: **read files in the project's folder**, **read and write files in the project's folder**, or **read and write a folder you choose**. You approve it once per script; if a later version asks for more, you're asked again. **No network at all in Phase 14.** A run is stopped after **10 seconds** (Settings → Scripts, from 1 second to 10 minutes) or **256 MB** of memory, with a message saying so; nothing it did is kept.
   - **Other ways:** allow the network with a permission; no permissions at all (scripts can only change the scene); trust every script fully, as Blender, Maya and 3ds Max do (their scripts can do anything the user can).
   - **Cost of changing later:** small to add a permission; removing one breaks the scripts that use it.
6. **What happens to scripts saved inside someone else's project?**
   - **Chosen:** a project's own scripts **don't run until you turn them on** for that project on your computer. Opening it shows a bar: "This project has 2 scripts. **Look at them** · **Turn them on**". Your own projects, saved on your computer, remember that you trusted them. (Blender does the same: its "Auto Run Python Scripts" is off as standard.)
   - **Other ways:** run them always; ask every time the project opens; never save scripts in projects.
   - **Cost of changing later:** small.
7. **How does a script say its name, what it does and what it needs?**
   - **Chosen:** a few **comment lines at the top** of the file (`-- name: Bevel hard edges`, `-- about: …`, `-- api: 1`, `-- asks: read and write the project folder`). The app reads them **without running anything**, so before installing a script you see what it is and what it asks for. The same lines work in LustScript and Lua.
   - **Other ways:** (a) Lust values at the top (`lock About about = new About { … };`), which would need Lust to learn to read the fixed values at the top of a file that also holds code; (b) a separate small file beside each script.
   - **Cost of changing later:** medium: every shared script's header would need rewriting (the app could keep reading the old lines).
8. **How is a script's command and its settings written?**
   - **Chosen:** in LustScript, a command is a **`component`** (Lust's existing word for a thing whose fields an editor shows): its fields are the command's **settings**, shown in Attributes like any tool's, and its **`run()`** method does the work. In Lua the same is `command("Name", { settings })` with a `run` function, mirroring Solis's Lua components. This works with Lust as it is today.
   - **Other ways:** (a) ask Lust for a word of its own (`command`), which reads better for modellers but waits on a Lust change; (b) plain methods with arguments, settings asked for in a box each time.
   - **Cost of changing later:** low if Lust adds `command` as another name for the same thing (old scripts keep working).
9. **Where do script commands show up?**
   - **Chosen:** in a new **Scripts** menu on the menu bar (after Window, before Help), and in **command search** (Space) like every command. Right-click one to **Add to the command palette** (the row of icons under the menus) or to **give it a key** of your own. Scripts get **no keys as standard**, and a key already taken is refused with the name of the command using it.
   - **Other ways:** put each script in the app's own menus (Mesh, UV…) where it fits; put them in the tool menu (E).
   - **Cost of changing later:** low.
10. **Where are scripts written and run?**
    - **Chosen:** one **Scripts** panel, a tab beside Coordinates at the bottom of the window, opened with **F11** or Window → Scripts. It has your scripts on the left, the script you're editing in the middle, and under it the **output** (what scripts print, and their mistakes) with a **console line** for trying one line at a time.
    - **Other ways:** a fifth layout, **Script**, on the menu bar beside Standard, UV Edit, Paint and Material, with the editor large (like Blender's Scripting workspace); a separate window; no editor inside the app at all.
    - **Cost of changing later:** low: the panel can move into a layout later.
11. **Must scripts be written inside the app?**
    - **Chosen:** no. The built-in editor colours the code, numbers the lines and underlines mistakes as you type. **Open in your own editor** opens the file in whatever program you use for `.lsc` or `.lua` files (Ledger, the Lust editor, say), and the app **reloads the script when the file changes**. Scripts in the built-in editor **save themselves** as you type; **Ctrl + S keeps meaning save the project**, everywhere.
    - **Other ways:** an outside editor only; Ctrl + S saves the script while you're typing in the editor (as code editors do), which would make one key mean two things.
    - **Cost of changing later:** low.
12. **Where are scripts kept, and how are they shared?**
    - **Chosen:** a new script goes in **your library** (a folder of your own, there in every project). **Keep with this project** copies it into the project file instead, so it travels with the project (file format 112). Scripts are shared as **single files**: **Export a script…** and **Install a script…**, which shows the script's name, what it asks for and its code before anything runs.
    - **Other ways:** scripts saved in projects as standard; script packs (several in one file); an online gallery (later, if wanted).
    - **Cost of changing later:** low.
13. **Can scripts run by themselves, when something happens?**
    - **Chosen:** yes, for three moments only: **a project opened**, **a project saved**, and **an export starting** (where a script can warn, or stop the export with a reason, but not change the scene: the export works on a copy). Each script that wants this must be switched on for it in the script manager ("Runs by itself when…"), and the status bar says when one ran. These runs get **2 seconds**, since they hold up opening, saving or exporting.
    - **Other ways:** more moments (on every selection change, every edit), which risks slowing the app down; none in Phase 14.
    - **Cost of changing later:** adding a moment is cheap; removing one breaks scripts.
14. **What do script calls look like, and what are they called?**
    - **Chosen:** the same shape as Solis's game calls ([Solis's game API guidelines](https://github.com/Rhyslos/solis/blob/main/docs/game-api-guidelines.md)): methods on things (`shape:bevel(…)`), a few named objects for the whole app (`Scene`, `Selection`, `Project`), fixed choices as Lust states (`Primitive.Cube`), readers named as nouns (`shape:name()`) and changes as verbs (`shape:setName(…)`). One table in the code lists every call once, and the LustScript files, the Lua names and the manual's reference are all made from it. The calls come from a folder named after the app, `aventail/` (`fetch "aventail/scene":Scene;`), and Lua reaches them as `aventail.Scene`, as Solis uses `solis/`.
    - **Other ways:** a name that never changes, like `app/` and `app.Scene`, so renaming the app doesn't touch scripts (renaming would otherwise keep the old name working beside the new one); calls shaped like Blender's Python (`bpy.ops.mesh.bevel(...)`).
    - **Cost of changing later:** the folder's name: low before scripts are shared, then it needs the old name kept working. The shape of the calls: high once scripts are shared.
15. **How do old scripts keep working as the app grows?**
    - **Chosen:** the calls have **one version number** (starting at 1), and a script says which it was written for (`-- api: 1`). New calls don't change the number. A change that would break scripts raises it, and the app keeps the previous version's calls working for at least one more version, with a note saying what to write now. A script for a newer version than the app says it needs a newer app.
    - **Other ways:** no number (simpler, but changes break scripts silently); three-part versions (1.2.0), more than a modelling app's scripts need.
    - **Cost of changing later:** low.
16. **Can a script be a tool you click and drag with in the view?**
    - **Chosen:** not in Phase 14. Script commands work like the app's other commands with settings: run once, and changing a setting in Attributes runs it again in place (one undo step, as Bevel's settings do). Clicks and drags in the view are for a later phase.
    - **Other ways:** let scripts hear the mouse in the view now (a lot more to build and keep safe).
    - **Cost of changing later:** adding it later costs nothing now.
17. **Which keys and mouse actions?**
    - **Chosen:** **F11** opens or closes the Scripts panel (3ds Max's key for its script window; free in this app). **Ctrl + Enter** in the editor runs the script (only the selected lines, when some are selected, as Maya's script editor does). **Esc** stops a running script, only while one runs. In the console line, **Enter** runs it and **Up**/**Down** go through earlier lines. **Ctrl + wheel** changes a script command's count setting, when its header names one ([decision 0048](../decisions/0048-ctrl-wheel-changes-a-tools-count.md)). No mouse actions in the view. Full list, with alternatives, under [Controls](#controls).
    - **Cost of changing later:** low; keys are kept in one list.

Further choices made while building milestone 1 (2026-11-17, also while the project lead was away; [decision 0301](../decisions/0301-script-runner.md)):

18. **What does running a script file do, before scripts become commands (milestone 4)?**
    - **Chosen:** it runs the file's own code from the top, then each command the file makes (a LustScript `component` with a `run()` method, or Lua's `command(…)` with a `run` function) **once, with its settings as they start**: in name order for LustScript, in the order made for Lua. So the plan's "Bevel hard edges" examples do something when run as a file.
    - **Other ways:** only the code at the top (commands do nothing until milestone 4); ask which command to run.
    - **Cost of changing later:** small; milestone 4 runs commands from the menu anyway.
19. **What's in the Scripts menu now?**
    - **Chosen:** **Run a script file…**, **Stop the script** (Esc, while one runs) and **Scripts panel** (shows or puts away the panel). The panel also shows by itself, beside Coordinates, whenever a script runs. Window → Scripts and F11 come with milestone 3, as planned.
    - **Other ways:** the panel only from the Window menu; no Stop entry (the status bar's button only).
    - **Cost of changing later:** tiny.
20. **Can the time and memory limits be changed yet?**
    - **Chosen:** not yet: **10 seconds and 256 MB**, fixed, until Settings → Scripts arrives with the script manager (milestone 4). Running out says how long or how much was allowed.
    - **Other ways:** a Settings page now (more to build before the runner is reviewed).
    - **Cost of changing later:** small.
21. **Where does the name `aventail` for the scripting folder come from?**
    - **Chosen:** a new `SCRIPT_NAME` in `branding.rs`, separate from `APP_ID` (which names the settings folder): a rename can change one without the other. The app hands it to scripting when it starts.
    - **Other ways:** reuse `APP_ID`; a name that never changes, such as `app` (choice 14's other way).
    - **Cost of changing later:** low before scripts are shared.
22. **Small shapes of version 1's reading calls**, settled while building them (the reference page lists them all):
    - **Chosen:** `Scene:shapes()` lists **every** object in the Objects panel, groups too (a group has no points). Positions are **relative to the group** a shape is in, as the Coordinates panel shows. `rotation()` gives degrees as a vec3: **heading in x, pitch in y, bank in z** (the app's own order). A `Polygon` gives its id, its **corners** (point ids in order), its **material's name** (not a slot number) and its **facing**; an `Edge` names its two points as **`a` and `b`** (`from` and `to` would clash with Lust's own words). `Mode` lists all eight of the app's modes. In Lua a state is its option's name as text (`"Polygons"`), as Lust hands states to programs, also kept as `aventail.Mode.Polygons`. Shapes are read-only values, one per shape, so `==` works.
    - **Other ways:** `shapes()` without groups; positions in the world; rotation as Lust's own `rotation` type; material slot numbers.
    - **Cost of changing later:** high once scripts are shared (scripts are written for them); low until then.

Further choices made while building milestone 2 (2026-11-17, also while the project lead was away; [decision 0302](../decisions/0302-scripts-change-the-scene.md)):

23. **What is the call that makes a ready-made shape called?**
    - **Chosen:** **`Scene:add(Primitive.Cube)`**. The plan's `create` is one of Lust's own words, so it can't name a call.
    - **Other ways:** `Scene:make(…)`; `Scene:newShape(…)`.
    - **Cost of changing later:** high once scripts are shared; tiny until then.
24. **In what space are a bevel's width, an extrude's distance and an inset's amount measured?**
    - **Chosen:** **as seen in the scene**, in centimetres, as the Bevel, Extrude and Inset tools measure: on a shape scaled to twice its size, a 2 mm bevel still looks 2 mm wide. (Points a script reads or builds are still in the shape's own space.)
    - **Other ways:** in the shape's own space, as its points are (a 2 mm bevel on a shape scaled ×2 would look 4 mm).
    - **Cost of changing later:** high once scripts are shared.
25. **How does `Scene:build` take its polygons, and how are their corners counted?**
    - **Chosen:** as a list of **`Polygon`s** (the same kind a shape hands back; only its `corners` are read), since Lust's native calls can't take lists of lists. Corners name points by their **place in the list of points, counting from 0**, as LustScript counts; that place is also each point's id in the new shape (in Lua too, where the first point is `points[1]` but is named 0).
    - **Other ways:** a flat list of corners and a list of how many each polygon has; counting from 1 in Lua.
    - **Cost of changing later:** high once scripts are shared.
26. **What happens to changes to a project while a script runs on it?**
    - **Chosen:** they're **turned away with a note** (the document is held): edits, undo and redo, and selecting all say a script is running and that Esc stops it. Looking around, reading panels and other tabs are free. A run in a tab you've left lands when it ends, and that tab doesn't go to sleep while it runs.
    - **Other ways:** queue the changes and make them after the landing (they could clash with what the script did); grey out the whole project's interface.
    - **Cost of changing later:** low.
27. **What if a script catches a refused change (`pcall`) and carries on?**
    - **Chosen:** **nothing it does lands**: every later change is refused too, and the run ends saying so. A refused change can leave the script's copy a little different from the project (an object id used up), so landing the rest could change the wrong things.
    - **Other ways:** land what came before the refusal; land everything but the refused change.
    - **Cost of changing later:** low.
28. **How much may one run change?**
    - **Chosen:** **100,000 changes** a run (the plan's number); a `Scene:build` of at most **100,000 points and 100,000 polygons** (1,000,000 corners in all), and **5,000,000** points and polygons built in a run, since what's built is kept in the app's memory, which the script's 256 MB doesn't count; a list given to one call (edges, polygons, ids) at most **100,000** items, as reading is chunked. Running past any of them stops the script with a message, and nothing lands.
    - **Other ways:** bigger or smaller numbers; a Settings page for them (milestone 4's Settings → Scripts could take them).
    - **Cost of changing later:** small.
29. **Small behaviours of the changing calls:**
    - **Chosen:** a new shape (`add`, `build`, `group`) **becomes the selection**, as when you make one yourself. Extrude and inset move **neighbours together as one piece**, extrude goes straight out, bevel is round (the tools' standard settings). `unwrap()` uses the **UV settings you have now**, on all the shape's polygons. `setMaterial` with a material the shape hasn't got **adds it**, with the colour another shape's material of that name has (or the standard grey). **Empty lists change nothing**, and so do edges already hard or already seams; elements a shape hasn't got are refused with their ids. `markHard` and `markSeams` only mark (clearing comes when a script needs it).
    - **Other ways:** new shapes left unselected; extrude settings as values the call takes; refusing empty lists.
    - **Cost of changing later:** low to medium once scripts are shared.

Further choices made while building milestone 3 (2026-11-17, also while the project lead was away; [decision 0303](../decisions/0303-scripts-panel.md)):

30. **Where is your script library?**
    - **Chosen:** a folder called `scripts` in the app's own data folder, beside its settings (on Windows in your AppData folder), the same for every project. Right-click a script → **Show the folder** opens it. The app writes its description files (`aventail/scene.lsc` and the rest) there too, for editors outside the app.
    - **Other ways:** a folder in your Documents (easier to find, but needs another library to find it on every system); a folder you choose in Settings.
    - **Cost of changing later:** small: scripts would be moved once.
31. **How does the console line work?**
    - **Chosen:** each line (or several, typed with Shift + Enter) is a **run of its own**: on a copy of the scene, landing as one undo step called "Console line". **Nothing carries over** from one line to the next (a variable made on one line is gone on the next). The language is chosen beside the line and **follows the script you open**; in LustScript every call is there without a `fetch`, and `end` is added. Up and Down go through the newest **100** lines (from the line's first and last row, so they still move the cursor in a line of several rows).
    - **Other ways:** keep one Lua for the whole session so variables carry over (against "one fresh sandbox a run", and a run can't hold the project between lines); Lua only.
    - **Cost of changing later:** medium for carrying variables over; small otherwise.
32. **What does Ctrl + Enter run when lines are selected?**
    - **Chosen:** the **whole lines** the selection touches, the rest of the script left out but its line numbers kept (so a mistake is shown at its own line), keeping the header lines and LustScript's `fetch` lines, with `end` added. The run is named after the script with its lines ("Bevel hard edges (lines 4 to 6)"). The **Run** button over the editor always runs the whole script.
    - **Other ways:** only the selected characters, as they are; the selection's lines without the `fetch` lines.
    - **Cost of changing later:** low.
33. **What if a script is changed in another program while you're typing in the editor?**
    - **Chosen:** with nothing waiting to be saved, it's simply loaded again (the output says so); with typing not yet saved, a bar asks **Load theirs** or **Keep mine**, and nothing is written over the other program's version meanwhile. The file is looked at once a second; a script is checked 0.4 s and saved 1 s after typing stops.
    - **Other ways:** the other program always wins; the editor always wins.
    - **Cost of changing later:** low.
34. **How are scripts made and found in the panel?**
    - **Chosen:** **+** over the list asks for a name and LustScript or Lua, and makes the script from a starter that says the selected shapes' names; the file is named as typed (`Bevel hard edges.lsc`). Double-click a script to edit it; right-click for **Edit**, **Run**, **Open in your own editor** and **Show the folder**. Renaming and removing come with the script manager (milestone 4).
    - **Other ways:** a New script… window; single-click to edit; rename and delete in the list now.
    - **Cost of changing later:** low.
35. **How much room does the Scripts panel get?**
    - **Chosen:** beside Coordinates the band is slim (64 px), too small for an editor, so showing the Scripts panel makes the band **320 px** tall (under half the window's height); putting it away gives Coordinates its slim band back. You can drag the band as usual.
    - **Other ways:** leave the band slim (drag it yourself); a Script layout with the editor large (choice 10's other way).
    - **Cost of changing later:** tiny.
36. **Which program does Open in your own editor use?**
    - **Chosen:** the one your computer uses for `.lsc` or `.lua` files (as double-clicking the file in its folder would); no setting in the app.
    - **Other ways:** a Settings entry naming your editor.
    - **Cost of changing later:** small.
37. **Does F11 work while typing?**
    - **Chosen:** yes: no text box uses F11, so it shows or puts away the panel from anywhere, also while typing in the editor (as Esc doesn't).
    - **Other ways:** only when not typing.
    - **Cost of changing later:** tiny.

Further choices made while building milestone 4 (2026-11-18, also while the project lead was away; [decision 0305](../decisions/0305-scripts-as-commands.md)):

38. **In Lua, is a number setting whole or with decimals?**
    - **Chosen:** by the number it starts at: `segments = 1` is a **whole number** (Attributes steps it 1, 2, 3), `width = 0.2` a **number with decimals**. Lua can't tell `1` from `1.0`, so a decimal setting must start at a number with decimals (the manual says so). LustScript says it itself (`int`, `float`).
    - **Other ways:** every Lua number setting has decimals except the header's `-- count:`; a header line giving each setting's kind.
    - **Cost of changing later:** small.
39. **What about a script that makes no commands?**
    - **Chosen:** it's **one command of its own**, named after the script, that runs the whole file (as Run does in the panel): so a simple script is in the Scripts menu, command search, the palette and keys too. A script with mistakes has no commands until it's mended (the script manager says why).
    - **Other ways:** only scripts with commands show as commands.
    - **Cost of changing later:** tiny.
40. **In what order are a Lua command's settings shown?**
    - **Chosen:** **by name** (a Lua table keeps no order). LustScript's show in the order written.
    - **Other ways:** a list of pairs (`{ {"width", 0.2}, … }`) to keep the order, which is harder to write.
    - **Cost of changing later:** small; scripts written either way could keep working.
41. **Which keys can a script command have?**
    - **Chosen:** any key, with or without Ctrl, Shift or Alt, that **nothing in the app uses anywhere** (every entry in the Keys window), and that **no other script command has**; Esc, Enter, Tab and Backspace on their own are refused too (typing and windows need them). The refusal names what uses the key. Pressed while nothing is typed, no window is open and no script runs. F6 to F10 and F12 are free, as are most Ctrl + Shift letters.
    - **Other ways:** allow a key that's only used in another mode (more keys, but one key meaning two things); keys only with Ctrl + Shift.
    - **Cost of changing later:** low.
42. **What range has Settings → Scripts?**
    - **Chosen:** time **1 second to 10 minutes** (the plan's), memory **32 MB to 2 GB** (standard 10 s and 256 MB). **Restore standard** puts the limits back but keeps keys, palette places and settings, which are your own.
    - **Other ways:** no memory setting; up to 4 GB.
    - **Cost of changing later:** tiny.
43. **How does Ctrl + wheel change a script's count?**
    - **Chosen:** while the command's settings show in Attributes (its run the last thing done), Ctrl + wheel in the view changes the setting its header's `-- count:` names, by one a notch, **from 1 to 1,000**, running the command again each time; it comes before a tool's own count, since the script's settings show first.
    - **Other ways:** only while no tool with a count is in use; any range (scripts can refuse what they don't like).
    - **Cost of changing later:** tiny.
44. **What does Remove in the script manager do?**
    - **Chosen:** asks first ("its file is deleted from your library, and can't be brought back"), then **deletes the file**; its keys, palette places and settings go with it. Rename keeps them.
    - **Other ways:** move it to a "removed" folder in the library, or the computer's bin, so it can be brought back.
    - **Cost of changing later:** small.
45. **What if a command fails after a setting is changed in Attributes?**
    - **Chosen:** the run before (with the last settings that worked) is **done again**, its settings go back in Attributes, and the status bar says why the new ones failed (the output has the mistake). So a setting typed wrong never loses what you had.
    - **Other ways:** leave the scene without the command's changes until a setting works.
    - **Cost of changing later:** low.
46. **When do a command's settings leave Attributes?**
    - **Chosen:** as a tool's Last edit does (decision 0120): once anything else is changed, undone or selected. Running the command again (from the menu or its key) brings them back, starting from the settings last used.
    - **Other ways:** keep them until another command runs.
    - **Cost of changing later:** low.

Further choices made while building milestone 5 (2026-11-18, also while the project lead was away; [decision 0307](../decisions/0307-keeping-sharing-and-permissions.md)):

47. **Can a project's scripts be edited where they are?**
    - **Chosen:** no: the Scripts panel lists them under **In this project** and shows one **read-only** (coloured, lines numbered). To change one, **Copy to your library**, edit it, and **Keep with this project** again (which updates the copy there). Keeping and taking out are each one step to undo.
    - **Other ways:** edit them in place in the editor (each pause in typing a change to the project, mixed in with modelling in undo); keep them in a folder beside the project file.
    - **Cost of changing later:** medium.
48. **How is "I trust this project's scripts" remembered?**
    - **Chosen:** by each script's **fingerprint** (a SHA-256 of its text), on this computer: turning a project's scripts on trusts exactly that code, in whatever project it's in, and a script changed in any way is off again until turned on. Keeping one of your own scripts with a project trusts it, so your own projects open with their scripts on.
    - **Other ways:** by the project file's place on disk (moving a project forgets; another file saved over it would inherit the trust); per project and fingerprint together.
    - **Cost of changing later:** small (people turn scripts on once more).
49. **When is a script asked about what it wants to reach?**
    - **Chosen:** when it's run and its header asks for something not yet allowed: a window lists what it asks, with **Allow and run** and **Don't run it**. The script manager shows each script's asks with **Allow…** and **Take back**. A script reaches only what its header asks for (a call reaching further is refused, naming the `-- asks:` line to add); the console line reaches nothing; a file run with Run a script file… is remembered by its place on disk.
    - **Other ways:** only in the script manager (runs fail until allowed); when it's installed.
    - **Cost of changing later:** low.
50. **Which folder does a script asking for "a folder you choose" use?**
    - **Chosen:** the one chosen in the asking window (**Choose…**), kept with the approval; such a script's files are all in that folder, even if it also asks for the project's.
    - **Other ways:** ask for the folder on every run; let paths say which folder (a mark at their start).
    - **Cost of changing later:** medium once scripts are shared.
51. **When are the files a script writes written?**
    - **Chosen:** when the run **ends well**, each file whole, before its scene changes land; a script that fails, is stopped or runs out of time writes nothing, and a script reading a file it wrote reads what it wrote. At most **1,000 files and 256 MB** a run; a file read is at most **16 MB** of text.
    - **Other ways:** write at once (a failure halfway leaves some files written); bigger or smaller limits.
    - **Cost of changing later:** low.
52. **Which paths may a script give?**
    - **Chosen:** written from inside the folder (`models/chair.glb`, with `/` or `\`), folders made as needed. Refused: `..`, whole paths and drive letters (anything with `:`), any **link** on the way (even one staying inside), names Windows keeps or can't hold, names over 255 bytes, paths over 1,024 bytes or 32 folders deep.
    - **Other ways:** follow links that stay inside the folder; allow whole paths that are inside it.
    - **Cost of changing later:** low to loosen, high to tighten once scripts are shared.
53. **How does `exportShapes` work?**
    - **Chosen:** `Project:exportShapes(shapes, path, Preset.Godot)` exports as the Export window does with that preset (shapes inside groups among them too, hidden ones left out), the path ending in the preset's ending (`.glb`, `.obj` or `.stl`), files that go beside it (an OBJ's `.mtl`, textures) next to it. It **gives nothing back** (LustScript would make every call keep what it gives).
    - **Other ways:** give back how many shapes went in; add the ending itself.
    - **Cost of changing later:** high once scripts are shared.
54. **What if you install a script with a name you already have?**
    - **Chosen:** it's added with a number ("Bevel 2.lsc"), never over yours (copying a project's script into your library does the same). Installing runs nothing and allows nothing.
    - **Other ways:** ask whether to replace yours.
    - **Cost of changing later:** tiny.
55. **Where are Install and Export?**
    - **Chosen:** **Scripts → Install a script…** and **Scripts → Export a script** (a list of yours, then the project's); **Export…** is also on a script's right-click menu in the Scripts panel.
    - **Other ways:** the File menu; dropping a file on the window (left for later, as the plan says).
    - **Cost of changing later:** tiny.
56. **Do a project's script commands get palette places and keys?**
    - **Chosen:** no: once turned on they're in the Scripts menu and command search (with "(this project's)" when one of yours has the same name) and their settings show in Attributes, but palette places and keys stay yours, the same in every project.
    - **Other ways:** let them have keys too (a key would do something different in each project).
    - **Cost of changing later:** low.
57. **What does the trust bar look like?**
    - **Chosen:** a bar under the menu bar: "This project has 2 scripts. They won't run until you turn them on." with **Look at them** (the Scripts panel, the first one shown), **Turn them on** and **Not now** (put away until the project's scripts change).
    - **Other ways:** a window when the project opens; only a note in the Scripts panel.
    - **Cost of changing later:** tiny.

Further choices made while building milestone 6 (2026-11-18, also while the project lead was away; [decision 0309](../decisions/0309-event-scripts-and-examples.md)):

58. **How does a script say it runs by itself at a moment?**
    - **Chosen:** with **Lust's own events**: the app declares `ProjectOpened`, `ProjectSaved` and `ExportStarting` in `aventail/moments.lsc`, fetched like the calls, and a component reacts with `on ProjectSaved() { … }`. In Lua, a command has a function named `on_` and the moment's name (`function Tidy:on_ProjectSaved() … end`), as Solis's Lua reacts to events. A component that only reacts (no `run()`) isn't a command.
    - **Other ways:** a header line (`-- runs when: saved`) running the whole file; plain functions at the top of the file.
    - **Cost of changing later:** high once scripts are shared.
59. **When exactly does each moment run?**
    - **Chosen:** **opened**: just after the project opens (what the script changes lands as one undo step, and the project then has unsaved changes); **saved**: just before the file is written, so what it changes is in the saved file; **export starting**: before anything is written, and before the file window asks where.
    - **Other ways:** saved after the file is written (changes would leave the project unsaved again); the export after you've chosen where.
    - **Cost of changing later:** low.
60. **Which settings does a command reacting to a moment use?**
    - **Chosen:** its settings **as last used** (from the menu or Attributes), as running it from the menu does; a component that only reacts uses its starting values.
    - **Other ways:** always the starting values.
    - **Cost of changing later:** tiny.
61. **Which exports are "an export starting"?**
    - **Chosen:** **every one**: the Export window's (and Export again), a kit export, Export to game project, and the export into the game project after every save. A script stopping that last one lets the **save** go on without the export.
    - **Other ways:** only the Export window's.
    - **Cost of changing later:** low.
62. **What can a script do while an export starts?**
    - **Chosen:** read what goes in (`Export:shapes()`, every shown object going into the file, and `Export:format()`, a new `Format` state: `Glb`, `Obj`, `Stl`), warn (`Script:warn`), and **`Export:cancel(reason)`**: the export doesn't happen and the status bar says the reason (the first reason counts, whatever the script does after). Changing the scene stops the script and keeps nothing; files it writes are written only if the export goes ahead. The `Export` calls work only then (a mistake anywhere else).
    - **Other ways:** stopping by giving back the reason; letting it change the copy that's exported.
    - **Cost of changing later:** high once scripts are shared.
63. **What about a script that asks to reach files you haven't allowed yet?**
    - **Chosen:** it's **left out at the moment**, with a note in the output saying to allow it in the script manager (nobody may be there to answer while a save waits).
    - **Other ways:** open the access window and make the save wait for the answer.
    - **Cost of changing later:** low.
64. **In what order, and what if one fails?**
    - **Chosen:** every script switched on for the moment (yours and the project's, once turned on) runs **one after another, in name order**; one that fails, is stopped with Esc or runs past its 2 seconds is said in the output and the status bar, and the save or export **goes on**. The Scripts panel isn't opened for these runs. The status bar says which ran: "Saved crate.avtl. Ran by itself as the project was saved: Tidy names." Opening or starting another project forgets a save waiting for its scripts.
    - **Other ways:** stop the save when a script fails; open the Scripts panel each time.
    - **Cost of changing later:** low.
65. **Where is a script switched on to run by itself?**
    - **Chosen:** in the **script manager**, under the script: "Runs by itself when: □ a project opens □ a project is saved □ an export starts", showing only the moments it reacts to (and any still switched on). **Off as standard.** Kept in the settings by script: yours by file name (following renames), a project's by its fingerprint, switched in the manager's In this project once its scripts are on.
    - **Other ways:** on as soon as a script reacts to a moment; a switch in the Scripts panel's right-click menu.
    - **Cost of changing later:** tiny.
66. **Can a project's scripts be turned off again?**
    - **Chosen:** yes: the script manager's **In this project** has **Turn them off** once they're on: they're off on this computer until turned on again (the trust bar stays away until the project's scripts change).
    - **Other ways:** in the Scripts panel's right-click menu; no way (as before).
    - **Cost of changing later:** tiny.
67. **Do scripts see Sculpt mode?**
    - **Chosen:** yes: `Mode.Sculpt` is a ninth option (before, scripts saw it as Model). A LustScript `check` over `Mode` must now have a `when Sculpt`; none are shared yet.
    - **Other ways:** keep seeing it as Model.
    - **Cost of changing later:** low now, high once scripts are shared.
68. **What are the example scripts, and how do you get one?**
    - **Chosen:** six, in LustScript: **Rename by pattern**, **Gear** (built from points and polygons, Ctrl + wheel for its teeth), **Check before export** (switched on, it stops a too-heavy export), **Export each shape**, **Select polygons facing up**, and **Tidy names** (the plan's five, and one showing a script that runs as a project is saved). **Scripts → Example scripts** copies one into your library (with a number beside one of yours of the same name) and opens it in the editor.
    - **Other ways:** Lua versions too; the examples offered in + New script; all put in a new library at once.
    - **Cost of changing later:** tiny.

## What the project lead asked for

- **Scripting so people can add their own tools and automate work** (the roadmap's Phase 14, 2026-10-18, [decision 0119](../decisions/0119-roadmap-after-the-uvs.md)).
- **LustScript, the project lead's own language, is the default; plain Lua works too** (decision 0119).
- **LustScript's licence must be permissive** (MIT or Apache-2.0) for the app to use it ([DESIGN.md §9](../../DESIGN.md#9-licensing-policy)).
- **Scripts change the scene only through the same commands as tools**, so every script edit can be undone and later shows in Edit layers (DESIGN.md §4.2–4.4, [decision 0006](../decisions/0006-base-plus-operation-stack.md)).
- **Asked on 2026-11-17** to add Phase 14's milestones.

## What LustScript is (studied 2026-11-17)

Lust ([github.com/Rhyslos/Lust](https://github.com/Rhyslos/Lust), alpha 73) is one language with two outputs: `.lst` files become Rust and are built into real programs; **`.lsc` files (LustScript) become plain Lua 5.1**, which Lua, LuaJIT and Luau all run. Lust is written in Rust.

- **Licence: MIT.** Permissive, allowed by §9 and `cargo deny`. Its view icons (Lucide, ISC) are only in the parts the app doesn't use.
- **It can be held inside another Rust program.** The compiler is a Rust library (`lust`, with `default-features = false` it needs nothing but Rust's own library and opens no files). The program hands it the files' paths and text; `lust::check_with` lists every mistake with its file, line and column, in plain words; `lust::to_lua_with` gives the Lua to run. `lust::Engine { folders }` names the program's own folders, whose files describe what the program offers with **`native` methods** (`method bevel(…) native;`): Lust checks every call against them before anything runs, and the program gives their bodies in a Lua table, `LUST_NATIVES`.
- **Running it** is up to the program: Lust doesn't run anything itself. A failure while running is a Lua error that `chunk.locate` turns back into the Lust file and line.
- **Components and events** (made for game engines) fit a modelling app's commands: a `component`'s fields come out as a list with their kinds and starting values (a bool, int, float, text, a state's options, a vec3, a colour), which is exactly what Attributes needs to show settings.
- **Lust's own `Files`, `Web` and the console** need Lua's `io` and `os`, which the sandbox doesn't give: calling them stops with Lust's own message saying they aren't available here. Scripts use the app's own calls instead, which check permissions.
- **State:** alpha, with releases tagged on GitHub and changes for programs listed version by version in its `ENGINES.md`. Not on crates.io yet. Things can change between alphas, so the app pins one tag.

So **LustScript can be used from the start; nothing waits on Lua first.** If Lust ever stops being usable here (its licence changes, or an alpha breaks something the app needs), plain `.lua` scripts keep working on their own, since they never go through the compiler.

## Following Solis

Solis, the project lead's game engine, already holds Lust for game code ([Solis decision 0016](https://github.com/Rhyslos/solis/blob/main/docs/decisions/0016-how-lust-joins-solis.md), [0028](https://github.com/Rhyslos/solis/blob/main/docs/decisions/0028-lust-runs-as-luau-while-editing.md)). The app does the same where it fits, so the two programs agree and a person who scripts one can script the other:

| | Solis | This app |
|---|---|---|
| Compiler | `lust` library, pinned, `default-features = false` | the same |
| Runs in | Luau through `mlua` | the same |
| What the program offers | description files with `native` methods, written from one table in Rust (`describe.rs`) into `solis/` | the same, into `aventail/` (choice 14) |
| Lua's face | `solis.World:say(…)` | `aventail.Scene:selected()` |
| Sandbox | no files, network or programs; 64 MB; deadlines per call | the same, 256 MB, 10 s a run (choice 5) |
| Checking | on a worker thread, one check at a time; the compiler can't be stopped, so a check past 30 s is given up on | the same |
| Calls' shape | Solis's game API guidelines | the same rules (choice 14) |
| Time limits | Luau's interrupt set directly (Solis's one `unsafe` module), because game code runs every frame and mlua's wrapper cost a fifth of its time | **mlua's own, safe `set_interrupt`**: scripts don't run every frame, so the cost doesn't matter, and the app's no-`unsafe` rule ([decision 0005](../decisions/0005-safety-lints.md)) stands |

Solis's code is the project lead's own (MIT, [Solis decision 0007](https://github.com/Rhyslos/solis/blob/main/docs/decisions/0007-mit-licence-and-separate-from-aventail.md)), so its sandbox (`luau.rs`), its checking in turns (`check_turns.rs`), its plain-words Lua errors (`lua_problem`) and its description table may be copied in and adapted, with a comment saying where each came from.

## What people can do with scripts

- **Run a script** from a file, from the Scripts panel or menu, or as a command with its own name.
- **Write and try scripts** in the Scripts panel: an editor that checks as you type, the output, and a console line for one line at a time.
- **Add their own commands:** a script becomes a named command in the Scripts menu and command search, with its settings in Attributes, optionally on the command palette and with a key of its own.
- **Make shapes:** generators that build a mesh from points and polygons (a gear, a staircase, a pipe run), recorded like any edit so they can be undone.
- **Batch work:** rename, clean up, check every shape before export, put materials on many shapes, export each shape to its own file (with permission to write files).
- **React to moments:** a script that checks the scene when an export starts, or tidies names when a project is saved (choice 13).
- **Keep and share scripts:** in their library, inside a project, or as a single file to give someone.

## Architecture

### Crates

- **`script`** (new, layer 2, Services, beside `fileio`; planned in [ARCHITECTURE.md](../../ARCHITECTURE.md) since decision 0011). It holds the Lust compiler and `mlua` with Luau, the sandbox, the description table and the files made from it, reading script headers, checking in turns, running a script against a `scene::Document`, and the script library on disk. It depends on `scene`, `mesh` and `words` (every message goes through the language layer, [decision 0132](../decisions/0132-one-language-layer.md)); permitted file access goes through `fileio` (same layer). It knows nothing of egui or the GPU, and can be tested without a window.
- **`ui`** gains the Scripts panel, the Scripts menu, the script manager, the trust bar, script commands in command search and on the command palette, and settings in Attributes.
- **`fileio`** saves a project's own scripts (file format 112) and the per-user script library and its approvals.
- **`config`** keeps the Scripts settings (time and memory limits) and keys given to script commands.

New outside libraries, all permissive: `lust` (MIT, from its Git repository, choice 3) and `mlua` with Luau (both MIT). Luau is built from C++ source, so the build needs a C++ compiler: already true on every CI machine (a C compiler is used for zstd).

### How a run works

1. **Read the header** (no running): name, what it's about, the API version, what it asks for, a count setting.
2. **Check and compile** on a worker thread, one check at a time (Solis's way): a LustScript file with the `aventail/` description files through `lust::check_with` and `to_lua_with`; a `.lua` file through Luau's own syntax check. Mistakes come back with the file, line and column.
3. **Describe:** run the script's start once in the sandbox with no scene and no permissions, to read its commands and their settings (a Lust `component`'s fields, or Lua's `command(…)` calls). Cached until the file changes.
4. **Run:** take a copy of the scene (`Scene::copy_with_results`, which shares meshes, so it's quick even for heavy shapes) into a `scene::Document` on a worker thread, and call the command's `run()` with its settings. Every change the script asks for is a `scene::Command` (or a recorded `Operation` inside one) carried out on the copy, so the script reads its own results as it goes.
5. **Land:** the interface thread carries out the same commands on the real document as **one undo step**, named after the script (`Document::execute_batch`, milestone 2). Commands are deterministic (DESIGN.md §4.2), so the result is the same; a test compares the two. A run that fails, is stopped, or runs out of time or memory lands nothing.
6. **Again with new settings:** changing a setting in Attributes takes the last run back and runs it again, the newest change winning, still one undo step (as built-in tools do with `Document::abandon`).

While a run is going, the project's scene is held: commands for it wait, and the status bar shows the script's name, a progress bar when the script reports one, and **Stop**.

### The sandbox

Copied from Solis's `luau.rs` and adapted:

- **One Luau per run**, made fresh, so nothing one script leaves behind reaches another.
- **No `io`, `os`, `package`, `require`, `debug`, `load` of bytecode** (text is always loaded as text: Luau would read text starting with a low byte as compiled code, which isn't safe from strangers), no `getfenv` or `setfenv`. The libraries and the app's calls are read-only.
- **Limits:** the run's time limit, checked by mlua's safe interrupt and sticky (once past, every check stops it, so `pcall` in a loop can't hold it off); a memory limit; at most 100,000 commands a run; printed output cut to 1,000 lines a run and 4 KB a line; mesh reads in chunks of at most 100,000 elements.
- **Permissions** are checked on every file call against what was approved. Paths are made absolute, links aren't followed out of the allowed folder, and `..` can't climb out of it.
- **Nothing from a script can crash the app:** the compiler's panics are caught and reported as a mistake (as Solis does), and every call into Luau returns an error, never a panic.

## The API

### Shape

The calls follow Solis's game API guidelines (choice 14):

- **Things have methods:** `shape:bevel(…)`, `shape:name()`.
- **Whole-app tools are a few named objects:** `Scene`, `Selection`, `Project`, `Script`. Their names avoid Lust's own (`Maths`, `Files`, `Time`, `Json`, `Web`, `Random`).
- **Fixed choices are states, never text:** `Primitive.Cube`, `Mode.Polygons`, `Permission.ProjectFolder`. A typo is a mistake before the script runs.
- **Readers are nouns, changes are verbs:** `position()` and `setPosition(…)`, `hide()`, `delete()`; yes/no questions read after `is`: `is shape:shown()?`.
- **One question, one call:** a polygon's points, material and facing come back together as one `Polygon`.
- **Sizes in centimetres**, the app's own unit ([decision 0068](../decisions/0068-pivot-editing-and-scene-units.md)); turns in degrees.
- **Elements by their ids:** points, edges and polygons are named by the mesh's stable ids, whole numbers a script keeps and hands back but doesn't do maths on.
- **One table, three faces:** `script::describe` lists every call once, with its owner, name, what it takes and gives, and its help text. The `aventail/*.lsc` description files, Lua's `aventail.` names and the manual's API reference page are made from it, so they can't disagree. A test fails if the reference page is out of date.

### Version 1

Milestone 1 brings the reading calls, milestone 2 the changing ones. The final list is settled in those milestones; the shape is:

| Owner | Reading | Changing |
|---|---|---|
| `Scene` | `shapes()` (all, in Objects order), `selected()`, `find(name)` | `add(Primitive kind)` (written `create` before milestone 2: a Lust word, choice 23), `build(name, points, polygons)` (a shape from points and polygons), `group(shapes)` |
| `Shape` | `name()`, `position()`, `rotation()`, `scale()`, `parent()`, `children()`, `shown()`, `pointCount()`, `points(from, count)`, `polygonCount()`, `polygons(from, count)`, `hardEdges()`, `seams()`, `materials()` | `setName`, `setPosition`, `setRotation`, `setScale`, `show`, `hide`, `delete`, `bevel(edges, width, segments)`, `extrude(polygons, distance)`, `inset(polygons, amount)`, `markHard(edges)`, `markSeams(edges)`, `unwrap()`, `setMaterial(polygons, name)`, `makeEditable()` |
| `Selection` | `mode()`, `points(shape)`, `edges(shape)`, `polygons(shape)` | `set(shapes)`, `setPolygons(shape, polygons)`, `setEdges(…)`, `setPoints(…)`, `clear()` |
| `Project` | `name()`, `folder()` | `readText(path)`, `writeText(path, text)`, `exportShapes(shapes, path, Preset)` (each needs a permission, milestone 5) |
| `Script` | `api()` | `say(text)` (`print` works too), `warn(text)`, `progress(done, what)` |

The app's other commands join version 1 when a script needs them, each one a call in the table with its settings as plain values (DESIGN.md §4.7's "doors": every action is a named command taking plain data).

### A command in LustScript

```
-- name: Bevel hard edges
-- about: Bevels every hard edge of the selected shapes.
-- api: 1
-- count: segments

fetch "aventail/scene":[Scene, Shape];

component BevelHardEdges {
    has {
        float width = 0.2;      -- centimetres
        int segments = 1;
    }

    -- Each selected shape, its hard edges rounded off.
    method run() {
        loop (Shape shape in Scene:selected()) {
            shape:bevel(shape:hardEdges(), width, segments);
        }
    }
}

pass BevelHardEdges;
end
```

### The same in Lua

```lua
-- name: Bevel hard edges
-- about: Bevels every hard edge of the selected shapes.
-- api: 1
-- count: segments

local Bevel = command("BevelHardEdges", { width = 0.2, segments = 1 })

function Bevel:run()
    for _, shape in ipairs(aventail.Scene:selected()) do
        shape:bevel(shape:hardEdges(), self.width, self.segments)
    end
end
```

Settings show in Attributes by their names in words (`width` as "Width"); a header line can give one a range and unit (`-- setting width: 0 to 10 cm`).

## Safety

Scripts are shared between strangers, so the app treats every script as untrusted (DESIGN.md §4.7):

- **Nothing outside the project** unless a permission was approved (choice 5): no files, no network, no other programs.
- **Time and memory limits** stop a runaway script with a plain message. The app never freezes: scripts run on another thread (choice 4), and Stop works at any moment.
- **Nothing is half-done:** a run that fails or is stopped lands nothing. A run that lands is one undo step.
- **Scripts in projects don't run until trusted** on this computer (choice 6). Installing a script shows its code and what it asks for first.
- **Event scripts** run only when switched on, for a short time, and can't change the scene while an export runs (choice 13).
- **Hostile scripts are tested:** endless loops, endless recursion, memory bombs (`string.rep`, huge tables), deep nesting, metatable tricks, attempts to reach `io`, `os`, `getfenv` and bytecode, paths climbing out of the allowed folder, and fuzzed headers and script files.

## Versioning

- The calls have one version number, starting at **1**; each script's header says which it was written for (choice 15).
- New calls don't change the number. A change that would break scripts raises it; the previous version's calls keep working for at least one more version, as a thin layer over the new ones, each giving a note in the output ("`shape:bevelEdges` is now `shape:bevel`: change it when you can").
- A script without an `api:` line is taken as version 1, with a note. A script for a newer version than the app is refused with "This script needs a newer version of the app (it was written for scripting version 3; this app has 2)."
- The LustScript description files say their version in their first line, which is also how they're recognised, as in Solis.
- Moving to a newer Lust is its own pull request, which reads Lust's ENGINES.md for each version skipped.

## Mistakes in plain words

Every message says which script, where, what went wrong and what to do (DESIGN.md guiding principle 7):

- **Before running:** Lust's own messages, which are already written this way (`bevel.lsc:12:5  width is a float, but this is text.`). Lua syntax errors are reworded the same way.
- **While running:** a failure points at the script's own line, not at the app's Lua (Lust's `chunk.locate`; for Lua, Solis's `lua_problem`). A call given the wrong thing names the call and the value: "`shape:bevel` wants a number for width, and was given nothing (nil)."
- **Limits:** "Bevel hard edges was stopped after 10 seconds, so nothing it did was kept. If it needs longer, raise the time limit in Settings → Scripts." Memory and the command count likewise.
- **Permissions:** "Bevel hard edges tried to write `out.txt`, but it isn't allowed to write files. If you trust it, allow it in the script manager."
- **Scene refusals** (a command the scene refuses, such as bevelling a shape that isn't editable) bring the scene's own reason and stop the run, landing nothing.
- Messages show in the Scripts panel's output, with the line clickable to go there in the editor; the status bar shows the first one.

## Controls

Following [decision 0028](../decisions/0028-ask-about-controls.md) and [decision 0048](../decisions/0048-ctrl-wheel-changes-a-tools-count.md). All chosen by Claude while the project lead was away (choice 17); none is built until reviewed.

| What | Chosen | Alternatives | Notes |
|---|---|---|---|
| Open or close the Scripts panel | **F11** | Shift + F11 (Cinema 4D's script manager key); no key, only Window → Scripts and the Scripts menu | F11 is 3ds Max's key for its script window. F6 to F12 are free in this app; F1 to F5 are the views. |
| Run the script in the editor | **Ctrl + Enter** (the selected lines only, when some are selected) | Ctrl + Shift + Enter; F5 (taken: a view) | Maya's script editor and many code editors use Ctrl + Enter. Only while typing in the editor. |
| Stop a running script | **Esc**, only while a script runs, and the **Stop** button | Ctrl + Break; the button only | Esc's other jobs (deselect, close) wait while a script holds the scene, so there's no clash; the status bar says "Esc stops it". |
| Console line | **Enter** runs the line, **Up** / **Down** go through earlier lines, **Shift + Enter** a new line | — | As every console does. |
| A script command's count | **Ctrl + wheel**, while its settings show, when its header names a count (`-- count: segments`) | none | Decision 0048: a tool with a count gets Ctrl + wheel. Added to `Tool::has_wheel_count` and `Workbench::wheel_count`'s script branch. |
| Keys for script commands | **None as standard**; right-click a command → **Give it a key…** and press one | Keys in the script's header | A key already taken is refused, naming the command using it. Kept in the settings file. |
| Save | **Ctrl + S stays save the project**; scripts in the editor save themselves | Ctrl + S saves the script while typing in the editor | Easy to confuse, so one key keeps one meaning (choice 11). |
| Undo while typing | **Ctrl + Z / Ctrl + Y** undo typing while the editor has the keyboard; outside it they undo scene changes | — | As every text field in the app already does. |
| Mouse | No new actions in the view. In the panel: double-click a script to edit it, right-click for its menu, click a mistake to go to its line | Dropping a script file on the window to install it | The drop is left out for now; Install a script… does it. |

Nothing comes from Halberd ([decision 0019](../decisions/0019-only-the-tool-keys-come-from-halberd.md)).

## The manual

Each milestone updates [the manual](../manual/) with what it builds, written for someone who has never written code:

- **Scripts** (`scripts.md`, milestone 1, grown by each milestone): what a script is and why you'd use one, running one, the Scripts panel, the output, mistakes and what they mean, limits and permissions, the library, project scripts and the trust bar, sharing, event scripts.
- **Your first script** (`your-first-script.md`, milestone 3): a step-by-step page that builds "bevel every hard edge of the selection" in LustScript, then the same in Lua, explaining each line.
- **Script reference** (`script-reference.md`, made from the description table, milestone 1 onwards): every call, what it takes and gives, an example, in LustScript and Lua side by side, with the version it arrived in.
- **Example scripts** (milestone 6): a handful in the app's starter scripts (rename by pattern, a gear generator, check before export, export each shape to its own file, select polygons facing up), each explained in the manual.
- **Concepts:** new terms in [concepts.md](../manual/concepts.md): script, LustScript, Lua, sandbox, permission, API version, console, event script.
- **Keyboard shortcuts** and **Settings** pages: the new keys and the Scripts page.

## Milestones

Six milestones, each one pull request. Each lists what it builds, its tests and its controls. Every milestone runs the whole suite, updates the manual, CHANGELOG, ARCHITECTURE.md and the crate READMEs, and adds its decision. The heavy-mesh check runs at the phase's end ([decision 0033](../decisions/0033-heavy-mesh-safeguard.md)), with a script that reads a 3-million-polygon shape in chunks and one that changes it.

### 1. The script runner

- The `script` crate: the pinned `lust` compiler and `mlua` with Luau (`deny.toml` allows Lust's repository); the sandbox and limits; headers read without running; checking in turns on a worker thread; the description table and the `aventail/` files made from it; version 1's **reading** calls and `Script`'s; running a `.lsc` or `.lua` file against a copy of the scene, its output and its mistakes coming back as messages.
- **Scripts** menu with **Run a script file…**; a first **Scripts** panel holding only the output (what scripts print, mistakes with file and line); **Stop** in the status bar.
- `RENAMING.md` gains the `aventail/` folder and the Lua `aventail` name; the folder's name comes from `branding.rs` (handed down by the app when scripting starts), never written in the code.
- **Tests:** a script reads names and counts in LustScript and Lua and prints them (normal); empty scripts, scripts with only a header, scripts with no `end`, a shape with no polygons (edge); every Lust mistake comes back with its place; runtime failures point at the script's line; endless loops, recursion, memory bombs, deep nesting, reaching `io`/`os`/`getfenv`/bytecode all fail with plain messages and the app carries on (failure, hostile); the time limit is sticky through `pcall`; fuzzed headers and script texts never panic (fuzz); the reference page matches the table (regression); a benchmark of compiling and starting a small script.
- **Controls:** Esc stops a running script.

#### What milestone 1 built

- **The `script` crate** (layer 2): `lust` at `v2.0.0-alpha.73` (no default features) and `mlua` 0.12 with Luau (held at 0.740, as Solis does); `deny.toml` allows Lust's repository as the one Git source. Modules: `header` (read without running), `describe` (the table, the description files and the reference page), `sandbox` and `prelude.lua` (from Solis's `luau.rs`), `turns` (from Solis's `check_turns.rs`), `compile`, `natives` and `values` (the calls' bodies, values read one at a time with plain mistakes, from Solis's `native_values.rs`), `problem` (from Solis's `lua_problem`, and Lust's `locate`), `output` and `run`.
- **Limits:** mlua's safe interrupt, read every 64 checks, sticky once past or once Stop is pressed; 10 s and 256 MB a run; 1,000 printed lines of at most 4 KB; chunks of 100,000 points or polygons; checks given up on after 30 s.
- **In the app:** the Scripts menu (Run a script file…, Stop the script, Scripts panel), the Scripts panel (output only, the newest 5,000 rows, with Clear), Stop and progress in the status bar, Esc while a script runs (and not while typing). `WorkbenchAction::RunScript`; `Workbench::run_script_file`; `AppInfo::script_name` from `branding::SCRIPT_NAME`; `FilePurpose::Script` when the system's file window can't open.
- **Tests** (`crates/script/src/*_tests.rs`, `crates/ui/src/workbench/scripting_tests.rs`, `crates/app/tests/script_reference.rs`): names and counts read in LustScript and Lua; every call answering in both; commands run once; empty, header-only and `end`-less scripts; Lust mistakes and runtime failures at the script's own line; endless loops, recursion, `pcall` and `xpcall` loops, memory bombs, deep nesting, reaching `io`, `os`, `debug`, `require`, `loadstring`, `getfenv`, the app's global and bytecode; nothing left from one run for the next; Stop and Esc; fuzzed headers (3,000) and scripts (150, short limits); the reference page against the table; a benchmark (a small script checked, compiled and run: about 12 ms for LustScript, 3 ms for Lua, in a debug build).
- **Notes for milestone 2:** the run now works on a `Snapshot` (a scene copy, the selection, the elements, the mode); milestone 2 turns it into a `scene::Document` and adds the commands, the held scene and the landing. The natives are one dispatcher per owner in `natives.rs`: a changing call is a new arm that runs a `scene::Command` on the copy and records it. Esc already waits for nothing else while a script runs; once the scene is held, the other jobs of Esc stay waiting, as planned.

### 2. Changing the scene

- Version 1's **changing** calls (creating, building shapes from points and polygons, moving, renaming, hiding, deleting, selecting, bevel, extrude, inset, hard edges, seams, unwrap, materials, make editable), each a `scene::Command` or a recorded operation, so edits show in Edit layers.
- A run lands as **one undo step** named after the script; nothing lands when it fails or is stopped; the project's scene is held while it runs.
- **Tests:** every changing call, through undo and redo (normal); a generator's mesh with bad polygons (out-of-range points, fewer than three corners, a million polygons) is refused with a reason (edge, failure); the copy and the real document end the same (checksum) after every call (round trip); a failing script lands nothing (regression); a script stopped halfway lands nothing; the commands-a-run limit; a benchmark landing 10,000 small changes.

#### What milestone 2 built

- **The changing calls** (`crates/script/src/changes.rs`): `Scene:add`, `build`, `group`; a shape's `setName`, `setPosition`, `setRotation`, `setScale`, `show`, `hide`, `delete`, `bevel`, `extrude`, `inset`, `markHard`, `markSeams`, `unwrap`, `setMaterial`, `makeEditable`; `Selection:set`, `setPoints`, `setEdges`, `setPolygons`, `clear`; the `Primitive` state. Each makes a `scene::Change` (a command, a recorded operation built as the tools build it, with anchors and stretch, or a selection change), carried out at once on the run's own `scene::Document` (made from the `Snapshot`, keeping no history) and kept. Lists are read by `values.rs` (`list`, `ids`, `edges`, `vec3s`, `corner_lists`, `shapes`, `option`), at most `MAX_LIST` (100,000) items.
- **Landing:** a run that ends well sends `Event::Land(Landing { changes, scene })` before it ends. `scene::Document::execute_batch` carries the changes out as one undo step (taking all back if one is refused, `Refused { at, why }`); the interface lands them named after the script and checks the scene matches the copy's. `Document::hold`/`release`/`is_held` hold the project while its script runs (`SceneError::Held`); every tab's scripts are taken in each frame (`SceneTabs::show`), a running script keeps its tab awake (`StaysAwake::RunningScript`), closing a tab stops its script (`Run` stops when dropped).
- **Limits:** `MAX_CHANGES` 100,000, `MAX_BUILD` 100,000, `MAX_CORNERS` 1,000,000, `MAX_BUILT` 5,000,000; a refused change spoils the run (`Lent::spoilt`).
- **Tests** (`crates/scene/src/document/batch_tests.rs`, `crates/script/src/changes_tests.rs`, `crates/ui/src/workbench/scripting_tests.rs`, `crates/ui/src/scene_tabs/tests.rs`): every changing call in Lua and LustScript landed on a held document, which ends as the copy did, through one undo and redo; each call on its own the same; 80 fuzzed scripts of changes, each landing as its copy ended; edits recorded in edit layers; deleting everything; bad generators (out-of-range corners, two corners, a point twice, not a number, a million polygons); elements a shape hasn't got; empty lists; a failing script, a refused change, a refusal caught with `pcall`, a script stopped halfway, too many changes and millions of shapes all land nothing; the project held while a script runs and Ctrl + Z saying why; a script in a tab left for another landing; landing 10,000 changes (about 13 ms in a debug build).
- **Notes for milestone 3:** the console line can run each line through the same `Run` (one landing per line); a run's `Landing` is all the editor needs to show what a script would change. For milestone 4, "run again with new settings" can take back the last step with `Document::undo` (the step has no merge key) or give `execute_batch` a key; the held document already keeps Attributes' live changes from racing a run.

### 3. The Scripts panel: editor and console

- The panel grows: your scripts on the left, the **editor** in the middle (colours for LustScript and Lua from a small tokenizer of our own, line numbers, mistakes underlined as you type, checked on the worker after a short pause), the output under it, and the **console line**.
- **New script** (LustScript, or Lua) from a short starter; **Open in your own editor**, and reloading when the file changes on disk (checked once a second, nothing new to install); scripts in the editor save themselves.
- **Tests:** the tokenizer on every kind of token, unfinished strings and comments, and fuzzed text (normal, edge, fuzz); a check while typing never runs code; Ctrl + Enter with and without a selection; console history; a file changed outside reloads once; a panel with a 10,000-line script stays smooth (benchmark).
- **Controls:** F11, Ctrl + Enter, the console's Enter, Up, Down and Shift + Enter, Ctrl + Z / Ctrl + Y in the editor.
- **Manual:** *Your first script*.

#### What milestone 3 built

- **In the `script` crate:** `tokens` (the colouring reader for both languages, `Token`, `TokenKind`), `pieces` (`only_lines` for Ctrl + Enter with a selection; `console_script` for the console line, with `describe::fetch_everything`), and `library` (`Library`: the folder, `prepare` writing the description files when they differ, `scripts`, `create` from `starter`; `file_name`, `save` through `fileio::project::safe_write`, `stamp`). `Problem` is hashable. `script` now depends on `fileio` (same layer).
- **The panel** (`crates/ui/src/scripts_panel.rs` and `scripts_panel/`): `list.rs` (the library, + New script, double-click, right-click), `editor.rs` (`Editor`: saving 1 s after typing, checking 0.4 s after on a thread of its own, looking at the file once a second, Load theirs or Keep mine, Ctrl + Enter, saving on close), `code_view.rs` (colours, underlines, line numbers, `Coloured` keeping each line laid out), `console.rs` (`Console`, `History`, `Entered`), `output.rs` (rows, a mistake in the open script clicked to go there). `Request`s (`Run`, `Outside`) are taken by `Workbench::poll_scripts`; `run_script_as` runs a script under a name, console mistakes moved to the typed line.
- **In the app:** Window → Scripts and F11 (`SCRIPTS_PANEL_KEY`, also in the Keys window); `layout::make_room_for_scripts` (320 px); `WorkbenchAction::OpenOutside` and `platform::open_outside`; `AppInfo::script_library` (`app::script_library`).
- **Tests** (`crates/script/src/tokens_tests.rs`, `pieces_tests.rs`, `library_tests.rs`, `crates/ui/src/scripts_panel/tests.rs`, `crates/ui/src/workbench/scripts_panel_tests.rs`): every kind of token in both languages, unfinished text and comments, 3,000 fuzzed texts; selected lines and console lines running, landing and placing mistakes; the library's description files, listing, starters that run in both languages, names refused, 3,000 fuzzed names, saving; console history and 20,000 fuzzed steps; lines for Ctrl + Enter; underlines; colouring covering the text; checking that never runs code; saving after a pause and on close; a file changed outside loaded once, and asked about while typing; F11 and Window → Scripts; Ctrl + Enter with and without a selection; typing and Ctrl + Z; the console's Enter, Shift + Enter, Up and Down; a new script made and opened; the band's room; a 10,000-line script (about 0.18 s a key and 0.06 s an unchanged frame for the whole window, in a debug build).
- **Notes for milestone 4:** the list's right-click menu is where Add to the command palette and Give it a key… go; the script manager can reuse `Library`. A script's commands for the Scripts menu can be read after the editor's check (the file's header is already read for the run's name). Settings changed in Attributes can run through `Request::Run` with the command chosen.

### 4. Scripts as commands

- A script's `component`s (or Lua `command(…)`s) become commands: in the **Scripts** menu and **command search**, with their **settings in Attributes**; changing a setting runs it again in place (one undo step); Ctrl + wheel for a count setting.
- **The script manager** (Scripts → Script manager…): your library's scripts with their names, what they're about and what they ask for; turn one off or on, remove it, reload it, show it in its folder. **Add to the command palette** and **Give it a key…** on a command's right-click menu.
- Version checks: a script for a newer version is refused, an older one runs with notes.
- **Tests:** settings of every kind shown and kept (normal); a script with no commands, two with the same name, a setting named like a reserved word (edge); a key already taken is refused (failure); live settings make one undo step however many changes (regression); script commands appear in command search and the palette; settings and keys survive a restart (round trip).
- **Controls:** Ctrl + wheel for counts, right-click on a script command.

#### What milestone 4 built

- **In the `script` crate:** `commands` (`commands_of`: the header, commands and their settings read by running the start once in a fresh sandbox against an empty scene, at most `DESCRIBE_LIMIT` (2 s), mistakes kept apart from notes; `CommandInfo`, `Setting`, `SettingKind`, `SettingValue`, `Choice`, `words_of`; at most 64 commands and 64 settings); `Run::start_command` runs one command with chosen settings (the run's start, then that command alone; a command no longer there is a mistake); an older scripting version gets a note. `Library::rename` and `Library::remove`.
- **In the app** (`crates/ui/src/script_commands.rs`): `ScriptSettings` (limits, palette, keys by name, scripts turned off, settings as last used; `checked` puts a damaged file right; saved as `scripts`, shared by tabs), `Registry` (the library read again on a thread of its own when a file's size or time changes, every 2 s), `ScriptCommand` (named by the header, "Script: Command" for several, files added to tell twins apart), `CommandRun`, `LiveCommand`, `KeyAsk`, `Manager`. `crates/ui/src/workbench/script_commands.rs`: running from the menu, search, palette and keys (`run_script_command`, `start_run`), the live command and running again (`change_script_settings`, `command_ended`: undo, run, redo on failure), Ctrl + wheel (`script_wheel`, before `wheel_count`'s tools). `script_manager.rs`: the manager window and the key window (`taken_by` against `shortcuts::bindings()`). `attributes/script_rows.rs`: the settings' rows. Settings → Scripts. `Icon::Script`; `PaletteCommand::Script`/`Unpin`; `SearchAction::Script`. The Scripts panel's list menu: Add to (or Take off) the command palette, Give it a key….
- **Tests** (`crates/script/src/commands_tests.rs`, `crates/ui/src/script_commands_tests.rs`, `crates/ui/src/workbench/script_commands_tests.rs`): every kind of setting read and set in both languages; values of the wrong kind or for settings gone; only the chosen command runs; no commands, components without `run`, two of one name, settings named like reserved words; mistakes, newer versions and slow starts giving no commands; reading reaching no scene; fuzzed settings (60) and settings files (3,000); the settings file's round trip, damaged values, renaming and removing; the library read, named, read again and forgotten; from the Scripts menu, search, the palette, the list's menu and a key; changing settings four times as one undo step; a failing change bringing the last good run back; settings leaving Attributes; Ctrl + wheel; taken keys refused (Ctrl + S, F11, another command's), the key window; a script with no commands; turning off; the manager's rename, remove and Keep it; settings shared by tabs and restored.
- **Notes for milestone 5:** `Registry` and `commands_of` read only your library; project scripts (file format 112) can reuse `commands_of` and `ScriptCommand` with their own `CommandKey` (a file inside the project, not in your library), and must wait for trust before their start is run, even to read their commands. Approvals can sit in `ScriptSettings` beside the keys. The manager's entry is where approving goes.

### 5. Keeping, sharing and permissions

- **Keep with this project** (file format 112), the **trust bar** for project scripts, remembered trust for your own projects.
- **Install a script…** and **Export a script…**, showing the code and what it asks for first.
- **Permissions:** the three file permissions, approved once per script and asked again when a script asks for more; `Project:readText`, `writeText` and `exportShapes`.
- **Tests:** a project's scripts saved and opened, older projects open as before, damaged and hostile script sections refused with a reason (round trip, fuzz); a trusted project's scripts run, an untrusted one's don't; every path trick (`..`, absolute paths, links out of the folder, very long names) refused; a write without permission refused and nothing written; an approval survives a restart, a bigger ask asks again.

#### What milestone 5 built

- **In `scene`:** `ProjectScript`, `Scene::scripts` (shared, `scripts_shared` for telling a change at a glance), `Command::SetScripts` (one undo step; `project_script::scripts_are_valid`: 64 scripts of 1 MB, script names only). **In `fileio`:** project format 113 (`project/v113.rs`, `ProjectV43::scripts`; 112 is sculpting's), sample `tests/data/sample-v113.avtl`.
- **In `script`:** `access` (`Access`, `place` checking every path, `Writes` holding what a run writes until it ends well, `read_text`, `write`, `check_write`; limits `MAX_READ_BYTES`, `MAX_WRITES`, `MAX_WRITE_BYTES`, `MAX_PATH_BYTES`, `MAX_NAME_BYTES`, `MAX_DEPTH`), `files` (`Project:readText`, `writeText`, `exportShapes` and the `Preset` state, through `fileio::export::export_bytes`), `Snapshot::access`, `fingerprint` (SHA-256, `sha2`), `Library::add` (installing, never over yours).
- **In the app** (`crates/ui/src/script_sharing.rs`): `ScriptId`, `Approval`, `ScriptSettings::{approvals, trusted}` with `needs_approval`, `approve`, `take_back`, `access_for`, `trust`; `ProjectScripts` (headers read at once, commands only once trusted, on a thread of their own); `Source`, `Then`, `AccessAsk`, `Installing`. `CommandKey::project`. `crates/ui/src/workbench/script_sharing.rs`: the trust bar, Keep with this project, Take out, Copy to your library, Install (the window showing the script), Export, the access window, the manager's approval lines. `scripts_panel/project_list.rs` (In this project) and `viewer.rs` (read-only). `start_run` takes the `Source`, and asks first when needed. `WorkbenchAction::{InstallScript, ExportScript, ChooseScriptFolder}`, `FilePurpose::{InstallScript, ExportScript, ScriptFolder}`; the app's `install_script`, `export_script`, `choose_script_folder`.
- **Tests** (`crates/scene/src/project_script.rs`, `crates/fileio/src/project/v113_tests.rs`, `crates/script/src/access_tests.rs`, `library_tests.rs`, `crates/ui/src/script_sharing_tests.rs`, `crates/ui/src/workbench/script_sharing_tests.rs`): scripts kept as one undo step and bad lists refused; a project's scripts saved and opened, older files opening without, damaged and hostile script sections refused, 200 flipped-byte files, the sample; every path trick (`..`, whole paths, drive letters, `:`, links out and in, reserved and long names, deep paths) and 3,000 fuzzed paths; reading and writing inside the folder; a write without permission refused with nothing written; an unsaved project; failing and timed-out runs writing nothing; caps on reading and writing; a chosen folder; exports with presets in both languages and refused ones; installing beside your own; fingerprints; approvals, bigger asks asking again, chosen folders, renames and removals, trust kept and damage put right, the settings' round trip; an untrusted project's scripts never read, a trusted one's commands; through the window: the trust bar and Turn them on, Not now, running an untrusted script refused, keeping and taking out with undo, the access window (Allow and run, Don't run it, asked once), the console reaching nothing, installing twice, exporting, copying in, the Scripts menu's Install, looking at a kept script.
- **Notes for milestone 6:** event scripts can use `ProjectScripts` and the library's `Registry` for "runs by itself when…", with the same trust: a project's event scripts don't run until it's trusted. The export-starting moment can hand scripts the export's shapes; `exportShapes` already writes through `Writes`, so a script stopping an export writes nothing. Approvals already name scripts by `ScriptId`; a switch per script for each moment can sit beside them in `ScriptSettings`.

### 6. Event scripts, examples and the gate

- Event scripts for **project opened**, **project saved** and **export starting** (warn or stop with a reason), switched on per script, 2 seconds each.
- **Example scripts** in the library's starters, each explained in the manual; the manual's script pages completed.
- The heavy-mesh check with scripts, and the phase gate.
- **Tests:** each moment runs its scripts once, in name order; a slow event script is stopped and the save or export goes on; an export stopped by a script says why and writes nothing; a script switched off doesn't run; the example scripts all run and do what the manual says.

#### What milestone 6 built

- **In the `script` crate:** `moments` (`Moment`, `ExportInfo`, `MOMENT_LIMIT`; the `Export` calls; what reacts to a moment, read from a component's `on` and a Lua command's `on_` functions), `Run::start_moment` (`Pick`: everything, one command, or a moment with its settings), `Snapshot::{moment, export}`, `Event::Cancelled`, `Described::moments`, `describe::HAPPENINGS` (the events in `aventail/moments.lsc`, `fetch_everything`, the reference's Moments section), the `Export` owner and `Format` state, `Mode::Sculpt`, a scene change refused while an export starts, files a stopped export's script wrote not written. `examples` (six `.lsc` files in `src/examples/`, `%folder%` for the scripting folder's name).
- **In the app:** `script_moments` (`Job`, `Current`, `MomentScript`, `Moments`, `OnCancel`) and `workbench/script_moments.rs` (`hold_for_moments`, `poll_moments`, `project_opened_moment`, `released_actions`, a save's export skipped through `save_export_request`); `ScriptSettings::moments` (`SavedMoment`; `runs_when`, `set_runs_when`, following renames and removals), `distrust`; the manager's switches and **Turn them off**; **Scripts → Example scripts** (`add_example`); a tab not in use hands on what its scripts released, and stays awake while something waits.
- **Tests** (`crates/script/src/moments_tests.rs`, `examples_tests.rs`, `crates/ui/src/workbench/script_moments_tests.rs`, `heavy_script_check.rs`): components reacting once each in name order, settings chosen, Lua's `on_` functions, reacting-only commands, reading and stopping an export in both languages, no scene changes then, the `Export` calls refused elsewhere, a slow script stopped after 2 s, files kept only when the export goes on, mistakes at the script's line, 40 fuzzed scripts; every example checking and doing what the manual says; through the window: a save waiting and its script landing first as one step, scripts switched or turned off not running, name order, an export stopped with its reason, a kit export and a save's export into the game project, a slow script stopped with the save going on, opening (a project's scripts only once turned on), a script asking for files left out, a replaced project forgetting its save, the manager's switch, Turn them off, the switches' round trip, Example scripts, Sculpt mode; the heavy scripting check.
- **Reading points and polygons made quicker:** the heavy check found reading every polygon of the three-million-polygon shape took 23.7 s, past the 10 s standard limit. A chunk is now packed into bytes in Rust and made into tables in Luau (`polygons_from` and `points_from` in `prelude.lua`), since making each table from Rust costs several protected calls into Luau: 11.1 s on the same busy machine, the same values (a test reads the sphere in uneven chunks against the mesh).
- **The heavy-mesh check** (release build, three million polygons): see the pull request ([#PR](https://github.com/Rhyslos/Aventail-3D/pulls)) for the table.

## Where things stand

| # | Milestone | PR | Decision | File format |
|---|---|---|---|---|
| 1 | The script runner | [#338](https://github.com/Rhyslos/Aventail-3D/pull/338) | [0301](../decisions/0301-script-runner.md) | 111 (unchanged) |
| 2 | Changing the scene | [#340](https://github.com/Rhyslos/Aventail-3D/pull/340) | [0302](../decisions/0302-scripts-change-the-scene.md) | 111 (unchanged) |
| 3 | The Scripts panel: editor and console | [#341](https://github.com/Rhyslos/Aventail-3D/pull/341) | [0303](../decisions/0303-scripts-panel.md) | 111 (unchanged) |
| 4 | Scripts as commands | [#343](https://github.com/Rhyslos/Aventail-3D/pull/343) | [0305](../decisions/0305-scripts-as-commands.md) | 111 (unchanged) |
| 5 | Keeping, sharing and permissions | [#345](https://github.com/Rhyslos/Aventail-3D/pull/345) | [0307](../decisions/0307-keeping-sharing-and-permissions.md) | 113 (112 is sculpting's) |
| 6 | Event scripts, examples and the gate | (this pull request) | [0309](../decisions/0309-event-scripts-and-examples.md) | 113 (unchanged) |

**Every milestone built** (2026-11-17 and 18), with their own choices added to the list above (18 to 22, 23 to 29, 30 to 37, 38 to 46, 47 to 57 and 58 to 68); every choice still waits for the project lead's review, and the gate waits for the project lead.

## The gate

**Done when:** the project lead writes a short script (say, "bevel every hard edge of the selection by 2 mm") in LustScript and again in Lua; it shows up as a command with its settings in Attributes, can be undone in one step, and can't touch files outside what it was allowed. A script with an endless loop is stopped with a plain message and the app never freezes.

## Later, not in Phase 14

- Script tools that hear clicks and drags in the view (choice 16).
- Network access with a permission, if ever wanted (choice 5).
- Script packs and an online gallery (choice 12).
- More moments for event scripts (choice 13).
- A Python bridge for those who want it and accept that it can't be sandboxed (decision 0011 left this open).
- Scripts driving the material graph and the painter (their commands join the table when wanted).
