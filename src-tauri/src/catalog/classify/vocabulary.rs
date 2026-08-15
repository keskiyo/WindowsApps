pub(super) static MEDIA_ASSOCIATIONS: &[&str] = &[
    ".mp3", ".flac", ".wav", ".ogg", ".opus", ".m4a", ".aac", ".wma", ".mkv", ".mp4", ".avi",
    ".mov", ".webm", ".m3u", ".m3u8",
];

pub(super) static EDITORS_ASSOCIATIONS: &[&str] = &[
    ".psd",
    ".ai",
    ".xcf",
    ".kra",
    ".svg",
    ".cdr",
    ".afphoto",
    ".afdesign",
    ".blend",
    ".fbx",
    ".raw",
    ".cr2",
    ".nef",
    ".dng",
];

pub(super) static DEVELOPMENT_ASSOCIATIONS: &[&str] = &[
    ".c", ".h", ".cpp", ".hpp", ".cs", ".java", ".py", ".rs", ".go", ".ts", ".tsx", ".jsx", ".php",
    ".rb", ".lua", ".sql", ".ipynb", ".sln", ".csproj",
];

pub(super) static FILE_CLOUD_ASSOCIATIONS: &[&str] = &[
    ".zip",
    ".rar",
    ".7z",
    ".tar",
    ".gz",
    ".bz2",
    ".xz",
    ".iso",
    ".cab",
    ".torrent",
    "url:magnet",
];

pub(super) static PRODUCTIVITY_ASSOCIATIONS: &[&str] = &[
    ".doc", ".docx", ".odt", ".xls", ".xlsx", ".ods", ".ppt", ".pptx", ".odp", ".pdf", ".epub",
    ".fb2", ".djvu",
];

pub(super) static BROWSERS_ASSOCIATIONS: &[&str] =
    &["url:http", "url:https", ".htm", ".html", ".xht"];

pub(super) static COMMUNICATION_ASSOCIATIONS: &[&str] = &[
    "url:mailto",
    "url:tg",
    "url:skype",
    "url:zoommtg",
    "url:discord",
    ".eml",
    ".vcf",
];

pub(super) static GAMES_ASSOCIATIONS: &[&str] = &["url:steam", "url:uplay", "url:battlenet"];

pub(super) static SYSTEM_ASSOCIATIONS: &[&str] = &[".vhd", ".vhdx", ".vmdk", ".ova", ".vbox"];

pub(super) static MEDIA: &[&str] = &[
    "media player",
    "music player",
    "video player",
    "audio player",
    "media center",
    "video converter",
    "audio editor",
    "медиаплеер",
    "медиапроигрыватель",
    "музыкальный проигрыватель",
    "видеопроигрыватель",
    "аудиопроигрыватель",
    "проигрыватель",
    "видеоконвертер",
];

pub(super) static BROWSERS: &[&str] = &[
    "web browser",
    "internet browser",
    "веб-браузер",
    "интернет-браузер",
    "веб браузер",
];

pub(super) static COMMUNICATION: &[&str] = &[
    "instant messaging",
    "messaging client",
    "chat client",
    "voice chat",
    "video conferencing",
    "email client",
    "e-mail",
    "mail client",
    "мессенджер",
    "обмен сообщениями",
    "видеоконференции",
    "почтовый клиент",
    "клиент электронной почты",
];

pub(super) static DEVELOPMENT: &[&str] = &[
    "code editor",
    "source code",
    "integrated development environment",
    "development environment",
    "compiler",
    "debugger",
    "interpreter",
    "version control",
    "database client",
    "terminal emulator",
    "package manager",
    "редактор кода",
    "исходный код",
    "среда разработки",
    "компилятор",
    "отладчик",
    "интерпретатор",
    "контроль версий",
    "эмулятор терминала",
];

pub(super) static EDITORS: &[&str] = &[
    "image editor",
    "photo editor",
    "graphics editor",
    "video editor",
    "vector graphics",
    "3d modeling",
    "digital painting",
    "графический редактор",
    "фоторедактор",
    "видеоредактор",
    "векторная графика",
    "трёхмерное моделирование",
    "цифровое рисование",
];

pub(super) static GAMES: &[&str] = &[
    "game launcher",
    "game client",
    "gaming platform",
    "игровой клиент",
    "игровая платформа",
    "лаунчер игр",
];

pub(super) static SECURITY: &[&str] = &[
    "antivirus",
    "anti-virus",
    "antimalware",
    "anti-malware",
    "endpoint security",
    "endpoint protection",
    "internet security",
    "password manager",
    "vpn",
    "vpn client",
    "proxy",
    "proxy client",
    "антивирус",
    "защита от вредоносных",
    "менеджер паролей",
    "хранилище паролей",
];

pub(super) static FILE_CLOUD: &[&str] = &[
    "file manager",
    "file archiver",
    "archive manager",
    "cloud storage",
    "file synchronization",
    "torrent client",
    "ftp client",
    "файловый менеджер",
    "архиватор",
    "менеджер архивов",
    "облачное хранилище",
    "синхронизация файлов",
    "торрент-клиент",
];

pub(super) static PRODUCTIVITY: &[&str] = &[
    "word processor",
    "spreadsheet",
    "presentation",
    "office suite",
    "pdf reader",
    "pdf viewer",
    "note taking",
    "текстовый процессор",
    "электронные таблицы",
    "офисный пакет",
    "просмотр pdf",
    "заметки",
];

pub(super) static UTILITIES: &[&str] = &[
    "device driver",
    "hardware monitoring",
    "benchmark",
    "remote desktop client",
    "screenshot",
    "clipboard manager",
    "драйвер устройства",
    "мониторинг оборудования",
    "удалённый рабочий стол",
    "снимок экрана",
    "буфер обмена",
];

pub(super) static AI: &[&str] = &[
    "ai assistant",
    "language model",
    "machine learning",
    "нейросеть",
    "языковая модель",
    "машинное обучение",
];

pub(super) static SYSTEM: &[&str] = &[
    "virtual machine",
    "disk partition",
    "partition manager",
    "disk imaging",
    "backup and recovery",
    "виртуальная машина",
    "разметка диска",
    "менеджер разделов",
    "образ диска",
    "резервное копирование",
];
