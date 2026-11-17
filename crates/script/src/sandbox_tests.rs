use super::*;

fn sandbox() -> Sandbox {
    Sandbox::new(
        Limits::default(),
        Arc::new(AtomicBool::new(false)),
        &crate::run::tests::names(),
    )
    .unwrap()
}

#[test]
fn libraries_and_the_apps_global_are_read_only_and_each_script_has_its_own_globals() {
    let sandbox = sandbox();
    sandbox.seal();
    let run = |text: &str, env: &Table| {
        as_text(sandbox.lua.load(text))
            .set_environment(env.clone())
            .exec()
            .map_err(|e| e.to_string())
    };
    let one = sandbox.env().unwrap();
    let two = sandbox.env().unwrap();
    for vandal in [
        "string.rep = nil",
        "table.insert = nil",
        "aventail.Scene = 1",
        "aventail.Mode.Points = 1",
        "getmetatable(aventail.Scene).__index = nil",
        "LUST_NATIVES.Scene_selected = print",
        "setmetatable(_G, {})",
        "setfenv(1, {})",
        "local x = getfenv(1)",
        "debug.info(1)",
        "os.clock()",
        "io.write(1)",
        "require('x')",
        "collectgarbage('stop')",
        "table.move({}, 1, 2e9, 1)",
    ] {
        assert!(run(vandal, &one).is_err(), "{vandal} should be refused");
    }
    run("pairs = nil; helper = 1", &one).unwrap();
    run("assert(pairs ~= nil); assert(helper == nil)", &two).unwrap();
    run("assert(collectgarbage('count') > 0)", &two).unwrap();
}

#[test]
fn the_clock_stops_once_past_and_keeps_saying_so() {
    let stop = Arc::new(AtomicBool::new(false));
    let clock = Clock::new(
        Limits {
            time: std::time::Duration::ZERO,
            ..Limits::default()
        },
        stop,
    );
    std::thread::sleep(std::time::Duration::from_millis(2));
    let first = (0..READ_EVERY + 1).find_map(|_| clock.verdict());
    assert_eq!(first, Some(Halt::TimeUp));
    // Sticky: every check after says so at once.
    assert_eq!(clock.verdict(), Some(Halt::TimeUp));
    assert_eq!(clock.halted(), Some(Halt::TimeUp));
}

#[test]
fn stop_is_seen_at_the_next_check() {
    let stop = Arc::new(AtomicBool::new(false));
    let clock = Clock::new(Limits::default(), stop.clone());
    assert_eq!(clock.verdict(), None);
    stop.store(true, Ordering::Relaxed);
    assert_eq!(clock.verdict(), Some(Halt::Asked));
}

#[test]
fn the_global_falls_back_when_the_folder_cant_be_a_lua_name() {
    assert_eq!(global_name("aventail"), "aventail");
    assert_eq!(global_name("my app"), "app");
    assert_eq!(global_name("3d"), "app");
    assert_eq!(global_name(""), "app");
}
