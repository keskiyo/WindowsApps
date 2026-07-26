//! App categorization: the final `classify_app` decision on a merged catalog record plus the
//! keyword, publisher, and install-path signals it weighs.

use super::{AppCategory, AppInfo, SourceKind};
/// Final category for a fully resolved catalog entry. Runs after deduplication on the merged
/// record, so it can weigh the strongest signals — the Steam source, a game-store install path,
/// and a known publisher — before falling back to name/path keywords. A user can always override
/// the category, so a rare miss is not costly.
pub(crate) fn classify_app(app: &AppInfo) -> AppCategory {
    // Strongest, near-certain signals first.
    if app.source_kind == SourceKind::Steam
        || is_game_install_path(app)
        || is_game_publisher(app.publisher.as_deref())
    {
        return AppCategory::Games;
    }
    if let Some(category) = publisher_category(app.publisher.as_deref()) {
        return category;
    }
    // Match keywords over the target exe too, not just the shortcut. The resolved executable name
    // is often the telling signal a neutral display name hides — `SotaVPN.exe`, `MongoDBCompass.exe`.
    classify_keywords(
        &format!(
            "{} {} {}",
            app.name,
            app.path,
            app.resolved_path.as_deref().unwrap_or_default()
        )
        .to_lowercase(),
    )
}

/// The install tree of a known game store. Matched on path/resolved/install so a Battle.net or
/// Epic game lands in Games even when its name carries no obvious game word.
fn is_game_install_path(app: &AppInfo) -> bool {
    const MARKERS: &[&str] = &[
        r"\steamapps\",
        r"\battle.net\",
        r"\epic games\",
        r"\riot games\",
        r"\gog galaxy\",
        r"\ea games\",
        r"\origin games\",
        r"\ubisoft\",
        r"\rockstar games\",
        r"\wargaming.net\",
        r"\hoyoplay\",
        r"\mihoyo\",
    ];
    [
        app.path.as_str(),
        app.resolved_path.as_deref().unwrap_or_default(),
        app.install_location.as_deref().unwrap_or_default(),
    ]
    .iter()
    .any(|value| {
        let lower = value.to_lowercase();
        MARKERS.iter().any(|marker| lower.contains(marker))
    })
}

/// Unambiguous game publishers — distinctive enough not to collide with non-game vendors (so no
/// bare "ea"/"riot" that would catch "Realtek"/"Patriot").
fn is_game_publisher(publisher: Option<&str>) -> bool {
    const PUBLISHERS: &[&str] = &[
        "blizzard",
        "valve",
        "riot games",
        "electronic arts",
        "ea games",
        "ea sports",
        "ubisoft",
        "epic games",
        "rockstar games",
        "bethesda",
        "cd projekt",
        "mihoyo",
        "hoyoverse",
        "cognosphere",
        "wargaming",
        "activision",
        "2k games",
        "square enix",
        "bandai namco",
        "capcom",
        "gaijin",
        "larian",
        "warhorse",
    ];
    publisher.is_some_and(|value| {
        let value = value.to_lowercase();
        PUBLISHERS.iter().any(|marker| value.contains(marker))
    })
}

/// A few unambiguous publishers that pin a category by themselves.
fn publisher_category(publisher: Option<&str>) -> Option<AppCategory> {
    let publisher = publisher?.to_lowercase();
    const EDITORS: &[&str] = &["adobe", "maxon", "blackmagic", "affinity", "corel"];
    const DEVELOPMENT: &[&str] = &[
        "jetbrains",
        "github, inc",
        "docker, inc",
        "gitkraken",
        "mongodb",
    ];
    if EDITORS.iter().any(|marker| publisher.contains(marker)) {
        return Some(AppCategory::Editors);
    }
    if DEVELOPMENT.iter().any(|marker| publisher.contains(marker)) {
        return Some(AppCategory::Development);
    }
    None
}

pub(super) fn classify(name: &str, path: &str) -> AppCategory {
    classify_keywords(&format!("{name} {path}").to_lowercase())
}

/// Category from curated keyword lists over a pre-lowercased blob (name + paths). First match wins.
fn classify_keywords(value: &str) -> AppCategory {
    let contains = |keywords: &[&str]| keywords.iter().any(|keyword| value.contains(keyword));

    if is_windows_feature(value) {
        AppCategory::WindowsFeatures
    } else if contains(&[
        "steam",
        "battle.net",
        "epic games",
        "gog",
        "game",
        "minecraft",
        "roblox",
        "backpack battles",
        "warcraft",
        "hearthstone",
        "dota",
        "valorant",
        "league of legends",
        "apex legends",
        "fortnite",
        "grand theft auto",
        "gta",
        "witcher",
        "cyberpunk",
        "overwatch",
        "diablo",
        "starcraft",
        "genshin",
        "honkai",
        "counter-strike",
        "cs2",
        "csgo",
        "pubg",
        "terraria",
        "stardew",
        "elden ring",
        "dark souls",
        "baldur",
        "warframe",
        "path of exile",
        "dead by daylight",
        "rainbow six",
        "call of duty",
        "warzone",
        "battlefield",
        "world of tanks",
        "war thunder",
        "palworld",
        "valheim",
        "clash of clans",
        "clash royale",
    ]) {
        AppCategory::Games
    } else if contains(&[
        "claude",
        "chatgpt",
        "openai",
        "codex",
        "ollama",
        "lm studio",
        "gemini",
        "copilot",
        "cursor",
        "ai agent",
        "perplexity",
        "mistral",
        "anythingllm",
        "msty",
        "opencode",
        "aider",
        "cline",
        "roo code",
    ]) {
        AppCategory::Ai
    } else if contains(&[
        "visual studio",
        "vscode",
        "code.exe",
        "rustrover",
        "pycharm",
        "webstorm",
        "intellij",
        "android studio",
        "git",
        "docker",
        "postman",
        "terminal",
        "sublime text",
        "notepad++",
        "node.js",
        "nodejs",
        "python",
        "github desktop",
        "sourcetree",
        "gitkraken",
        "dbeaver",
        "tableplus",
        "heidisql",
        "insomnia",
        "clion",
        "datagrip",
        "phpstorm",
        "cmake",
        "mongodb",
        "studio 3t",
        "robo 3t",
        "dbgate",
        "beekeeper",
    ]) {
        AppCategory::Development
    } else if contains(&[
        "photoshop",
        "illustrator",
        "lightroom",
        "figma",
        "gimp",
        "inkscape",
        "blender",
        "paint",
        "canva",
        "davinci resolve",
        "premiere",
        "after effects",
        "affinity",
        "krita",
        "clip studio",
        "capcut",
        "darktable",
    ]) {
        AppCategory::Editors
    } else if contains(&[
        "chrome", "firefox", "edge", "opera", "brave", "vivaldi", "browser", "yandex", "chromium",
        "thorium",
    ]) {
        AppCategory::Browsers
    } else if contains(&[
        "vlc",
        "spotify",
        "foobar",
        "audacity",
        "obs",
        "media player",
        "music",
        "video",
        "potplayer",
        "mpc-hc",
        "mpc-be",
        "kmplayer",
        "aimp",
        "winamp",
        "itunes",
        "plex",
        "kodi",
        "deezer",
        "tidal",
        "jellyfin",
    ]) {
        AppCategory::Media
    } else if contains(&[
        "discord",
        "telegram",
        "whatsapp",
        "slack",
        "teams",
        "zoom",
        "skype",
        "signal",
        "viber",
        "wechat",
        "element",
        "thunderbird",
    ]) {
        AppCategory::Communication
    } else if contains(&[
        "character map",
        "command prompt",
        "powershell",
        "windows terminal",
        "task manager",
        "таблица символов",
        "командная строка",
        "диспетчер задач",
    ]) {
        AppCategory::System
    } else if contains(&[
        "7-zip",
        "winrar",
        "everything",
        "powertoys",
        "rufus",
        "verifier",
        "configure",
        "utility",
        "uninstaller",
        // VPN / proxy clients.
        "vpn",
        "proxy",
        "wireguard",
        "openvpn",
        "hiddify",
        "nekoray",
        "nekobox",
        "clash verge",
        "clashx",
        "mihomo",
        "v2ray",
        "xray",
        "sing-box",
        "shadowsocks",
        "outline",
        "cloudflare warp",
        "mullvad",
        "proton vpn",
        "nordvpn",
        "expressvpn",
        "surfshark",
        "windscribe",
        "ipvanish",
        "tunnelbear",
        "psiphon",
        // System / hardware tools.
        "cpu-z",
        "gpu-z",
        "hwinfo",
        "hwmonitor",
        "crystaldisk",
        "aida64",
        "msi afterburner",
        "rivatuner",
        "treesize",
        "windirstat",
        "ccleaner",
        "revo uninstaller",
        "display driver uninstaller",
        "victoria",
        "memtest",
        "furmark",
        "winmtr",
    ]) {
        AppCategory::Utilities
    } else {
        AppCategory::Other
    }
}

fn is_windows_feature(value: &str) -> bool {
    const FEATURES: &[&str] = &[
        // Windows shell, inbox applications and accessibility tools.
        "file explorer",
        "explorer.exe",
        "проводник",
        "snipping tool",
        "snippingtool.exe",
        "microsoft.screensketch",
        "ножницы",
        "get help",
        "microsoft.gethelp",
        "техническая поддержка",
        "remote desktop connection",
        "mstsc.exe",
        "подключение к удаленному рабочему столу",
        "calculator",
        "microsoft.windowscalculator",
        "калькулятор",
        "notepad",
        "microsoft.windowsnotepad",
        "блокнот",
        "microsoft paint",
        "mspaint.exe",
        "microsoft.paint",
        "камера",
        "microsoft.windowscamera",
        "clock",
        "microsoft.windowsalarms",
        "часы",
        "voice recorder",
        "sound recorder",
        "microsoft.windowssoundrecorder",
        "запись голоса",
        "звукозапись",
        "magnifier",
        "magnify.exe",
        "экранная лупа",
        "on-screen keyboard",
        "osk.exe",
        "экранная клавиатура",
        "narrator",
        "narrator.exe",
        "экранный диктор",
        "quick assist",
        "microsoftcorporationii.quickassist",
        "быстрая помощь",
        "windows security",
        "microsoft.sechealthui",
        "безопасность windows",
        "windows settings",
        "systemsettings.exe",
        "параметры",
        "microsoft.windows.administrativetools",
        "инструменты windows",
        "windows backup",
        "windowsbackup",
        "архивация windows",
        "microsoft.windowsstore",
        "microsoft store",
        "microsoft.windows.photos",
        "фотографии",
        "microsoft.bingweather",
        "погода",
        "microsoft.bingnews",
        "новости",
        "microsoft.microsoftstickynotes",
        "sticky notes",
        "записки",
        "microsoft.yourphone",
        "phone link",
        "связь с телефоном",
        "microsoft.windowsfeedbackhub",
        "feedback hub",
        "центр отзывов",
        "microsoft.windows.shell.rundialog",
        "выполнить",
        "windows media player legacy",
        "steps recorder",
        "psr.exe",
        "средство записи действий",
        "memory diagnostics tool",
        "mdsched.exe",
        "средство проверки памяти windows",
        "recoverydrive",
        "recoverydrive.exe",
        "диск восстановления",
        "iscsi initiator",
        "iscsicpl.exe",
        "инициатор iscsi",
        // Management consoles and administrative tools.
        "computer management",
        "compmgmt.msc",
        "управление компьютером",
        "print management",
        "printmanagement.msc",
        "управление печатью",
        "device manager",
        "devmgmt.msc",
        "диспетчер устройств",
        "disk management",
        "diskmgmt.msc",
        "управление дисками",
        "event viewer",
        "eventvwr.msc",
        "просмотр событий",
        "task scheduler",
        "taskschd.msc",
        "планировщик задач",
        "планировщик заданий",
        "local security policy",
        "secpol.msc",
        "локальная политика безопасности",
        "group policy",
        "gpedit.msc",
        "групповая политика",
        "system configuration",
        "msconfig.exe",
        "конфигурация системы",
        "resource monitor",
        "resmon.exe",
        "монитор ресурсов",
        "performance monitor",
        "perfmon.msc",
        "системный монитор",
        "component services",
        "comexp.msc",
        "службы компонентов",
        "odbc data sources",
        "odbcad32.exe",
        "источники данных odbc",
        "system information",
        "msinfo32.exe",
        "сведения о системе",
        "control panel",
        "control.exe",
        "панель управления",
        "administrative tools",
        "windows tools",
        "администрирование",
        "средства windows",
        "registry editor",
        "regedit.exe",
        "редактор реестра",
        "disk cleanup",
        "cleanmgr.exe",
        "очистка диска",
        "defragment",
        "dfrgui.exe",
        "дефрагментация",
        "windows defender firewall",
        "wf.msc",
        "брандмауэр",
        "system restore",
        "rstrui.exe",
        "восстановление системы",
        "services.msc",
        "службы",
        "character map",
        "charmap.exe",
        "таблица символов",
        "command prompt",
        "cmd.exe",
        "командная строка",
        "windows powershell",
        "powershell.exe",
        "windows terminal",
        "microsoft.windowsterminal",
        "task manager",
        "taskmgr.exe",
        "диспетчер задач",
    ];

    FEATURES.iter().any(|feature| value.contains(feature))
}
