// Microbenchmarks for the TZ/Calendar ID storage and lookup strategies.
//
// On `main`, ZonedDateTime stores tz_id and calendar_id as inline `String`
// fields — no lookup at all.  This file benchmarks those inline-string costs
// alongside binary-search and HashMap alternatives, giving a fair cross-branch
// comparison when run against the `storage-redesign` results.
//
// Run with: `cargo bench --bench tz_lookup`

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Static TZ data (598 IANA timezone IDs, same set as storage-redesign)
// ---------------------------------------------------------------------------

/// Canonical order — element index is the logical "stored index" used on
/// the storage-redesign branch.
const TZ_CANONICAL: &[&str] = &[
    "Africa/Abidjan", "Africa/Accra", "Africa/Addis_Ababa", "Africa/Algiers",
    "Africa/Asmara", "Africa/Asmera", "Africa/Bamako", "Africa/Bangui",
    "Africa/Banjul", "Africa/Bissau", "Africa/Blantyre", "Africa/Brazzaville",
    "Africa/Bujumbura", "Africa/Cairo", "Africa/Casablanca", "Africa/Ceuta",
    "Africa/Conakry", "Africa/Dakar", "Africa/Dar_es_Salaam", "Africa/Djibouti",
    "Africa/Douala", "Africa/El_Aaiun", "Africa/Freetown", "Africa/Gaborone",
    "Africa/Harare", "Africa/Johannesburg", "Africa/Juba", "Africa/Kampala",
    "Africa/Khartoum", "Africa/Kigali", "Africa/Kinshasa", "Africa/Lagos",
    "Africa/Libreville", "Africa/Lome", "Africa/Luanda", "Africa/Lubumbashi",
    "Africa/Lusaka", "Africa/Malabo", "Africa/Maputo", "Africa/Maseru",
    "Africa/Mbabane", "Africa/Mogadishu", "Africa/Monrovia", "Africa/Nairobi",
    "Africa/Ndjamena", "Africa/Niamey", "Africa/Nouakchott", "Africa/Ouagadougou",
    "Africa/Porto-Novo", "Africa/Sao_Tome", "Africa/Timbuktu", "Africa/Tripoli",
    "Africa/Tunis", "Africa/Windhoek",
    "America/Adak", "America/Anchorage", "America/Anguilla", "America/Antigua",
    "America/Araguaina", "America/Argentina/Buenos_Aires",
    "America/Argentina/Catamarca", "America/Argentina/ComodRivadavia",
    "America/Argentina/Cordoba", "America/Argentina/Jujuy",
    "America/Argentina/La_Rioja", "America/Argentina/Mendoza",
    "America/Argentina/Rio_Gallegos", "America/Argentina/Salta",
    "America/Argentina/San_Juan", "America/Argentina/San_Luis",
    "America/Argentina/Tucuman", "America/Argentina/Ushuaia",
    "America/Aruba", "America/Asuncion", "America/Atikokan",
    "America/Atka", "America/Bahia", "America/Bahia_Banderas",
    "America/Barbados", "America/Belem", "America/Belize",
    "America/Blanc-Sablon", "America/Boa_Vista", "America/Bogota",
    "America/Boise", "America/Buenos_Aires", "America/Cambridge_Bay",
    "America/Campo_Grande", "America/Cancun", "America/Caracas",
    "America/Catamarca", "America/Cayenne", "America/Cayman",
    "America/Chicago", "America/Chihuahua", "America/Ciudad_Juarez",
    "America/Coral_Harbour", "America/Cordoba", "America/Costa_Rica",
    "America/Creston", "America/Cuiaba", "America/Curacao",
    "America/Danmarkshavn", "America/Dawson", "America/Dawson_Creek",
    "America/Denver", "America/Detroit", "America/Dominica",
    "America/Edmonton", "America/Eirunepe", "America/El_Salvador",
    "America/Ensenada", "America/Fort_Nelson", "America/Fort_Wayne",
    "America/Fortaleza", "America/Glace_Bay", "America/Godthab",
    "America/Goose_Bay", "America/Grand_Turk", "America/Grenada",
    "America/Guadeloupe", "America/Guatemala", "America/Guayaquil",
    "America/Guyana", "America/Halifax", "America/Havana",
    "America/Hermosillo", "America/Indiana/Indianapolis",
    "America/Indiana/Knox", "America/Indiana/Marengo",
    "America/Indiana/Petersburg", "America/Indiana/Tell_City",
    "America/Indiana/Vevay", "America/Indiana/Vincennes",
    "America/Indiana/Winamac", "America/Indianapolis", "America/Inuvik",
    "America/Iqaluit", "America/Jamaica", "America/Jujuy",
    "America/Juneau", "America/Kentucky/Louisville",
    "America/Kentucky/Monticello", "America/Knox_IN", "America/Kralendijk",
    "America/La_Paz", "America/Lima", "America/Los_Angeles",
    "America/Louisville", "America/Lower_Princes", "America/Maceio",
    "America/Managua", "America/Manaus", "America/Marigot",
    "America/Martinique", "America/Matamoros", "America/Mazatlan",
    "America/Mendoza", "America/Menominee", "America/Merida",
    "America/Metlakatla", "America/Mexico_City", "America/Miquelon",
    "America/Moncton", "America/Monterrey", "America/Montevideo",
    "America/Montreal", "America/Montserrat", "America/Nassau",
    "America/New_York", "America/Nipigon", "America/Nome",
    "America/Noronha", "America/North_Dakota/Beulah",
    "America/North_Dakota/Center", "America/North_Dakota/New_Salem",
    "America/Nuuk", "America/Ojinaga", "America/Panama",
    "America/Pangnirtung", "America/Paramaribo", "America/Phoenix",
    "America/Port-au-Prince", "America/Port_of_Spain", "America/Porto_Acre",
    "America/Porto_Velho", "America/Puerto_Rico", "America/Punta_Arenas",
    "America/Rainy_River", "America/Rankin_Inlet", "America/Recife",
    "America/Regina", "America/Resolute", "America/Rio_Branco",
    "America/Rosario", "America/Santa_Isabel", "America/Santarem",
    "America/Santiago", "America/Santo_Domingo", "America/Sao_Paulo",
    "America/Scoresbysund", "America/Shiprock", "America/Sitka",
    "America/St_Barthelemy", "America/St_Johns", "America/St_Kitts",
    "America/St_Lucia", "America/St_Thomas", "America/St_Vincent",
    "America/Swift_Current", "America/Tegucigalpa", "America/Thule",
    "America/Thunder_Bay", "America/Tijuana", "America/Toronto",
    "America/Tortola", "America/Vancouver", "America/Virgin",
    "America/Whitehorse", "America/Winnipeg", "America/Yakutat",
    "America/Yellowknife",
    "Antarctica/Casey", "Antarctica/Davis", "Antarctica/DumontDUrville",
    "Antarctica/Macquarie", "Antarctica/Mawson", "Antarctica/McMurdo",
    "Antarctica/Palmer", "Antarctica/Rothera", "Antarctica/South_Pole",
    "Antarctica/Syowa", "Antarctica/Troll", "Antarctica/Vostok",
    "Arctic/Longyearbyen",
    "Asia/Aden", "Asia/Almaty", "Asia/Amman", "Asia/Anadyr",
    "Asia/Aqtau", "Asia/Aqtobe", "Asia/Ashgabat", "Asia/Ashkhabad",
    "Asia/Atyrau", "Asia/Baghdad", "Asia/Bahrain", "Asia/Baku",
    "Asia/Bangkok", "Asia/Barnaul", "Asia/Beirut", "Asia/Bishkek",
    "Asia/Brunei", "Asia/Calcutta", "Asia/Chita", "Asia/Choibalsan",
    "Asia/Chongqing", "Asia/Chungking", "Asia/Colombo", "Asia/Dacca",
    "Asia/Damascus", "Asia/Dhaka", "Asia/Dili", "Asia/Dubai",
    "Asia/Dushanbe", "Asia/Famagusta", "Asia/Gaza", "Asia/Harbin",
    "Asia/Hebron", "Asia/Ho_Chi_Minh", "Asia/Hong_Kong", "Asia/Hovd",
    "Asia/Irkutsk", "Asia/Istanbul", "Asia/Jakarta", "Asia/Jayapura",
    "Asia/Jerusalem", "Asia/Kabul", "Asia/Kamchatka", "Asia/Karachi",
    "Asia/Kashgar", "Asia/Kathmandu", "Asia/Katmandu", "Asia/Khandyga",
    "Asia/Kolkata", "Asia/Krasnoyarsk", "Asia/Kuala_Lumpur", "Asia/Kuching",
    "Asia/Kuwait", "Asia/Macao", "Asia/Macau", "Asia/Magadan",
    "Asia/Makassar", "Asia/Manila", "Asia/Muscat", "Asia/Nicosia",
    "Asia/Novokuznetsk", "Asia/Novosibirsk", "Asia/Omsk", "Asia/Oral",
    "Asia/Phnom_Penh", "Asia/Pontianak", "Asia/Pyongyang", "Asia/Qatar",
    "Asia/Qostanay", "Asia/Qyzylorda", "Asia/Rangoon", "Asia/Riyadh",
    "Asia/Saigon", "Asia/Sakhalin", "Asia/Samarkand", "Asia/Seoul",
    "Asia/Shanghai", "Asia/Singapore", "Asia/Srednekolymsk", "Asia/Taipei",
    "Asia/Tashkent", "Asia/Tbilisi", "Asia/Tehran", "Asia/Tel_Aviv",
    "Asia/Thimbu", "Asia/Thimphu", "Asia/Tokyo", "Asia/Tomsk",
    "Asia/Ujung_Pandang", "Asia/Ulaanbaatar", "Asia/Ulan_Bator",
    "Asia/Urumqi", "Asia/Ust-Nera", "Asia/Vientiane", "Asia/Vladivostok",
    "Asia/Yakutsk", "Asia/Yangon", "Asia/Yekaterinburg", "Asia/Yerevan",
    "Atlantic/Azores", "Atlantic/Bermuda", "Atlantic/Canary",
    "Atlantic/Cape_Verde", "Atlantic/Faeroe", "Atlantic/Faroe",
    "Atlantic/Jan_Mayen", "Atlantic/Madeira", "Atlantic/Reykjavik",
    "Atlantic/South_Georgia", "Atlantic/St_Helena", "Atlantic/Stanley",
    "Australia/ACT", "Australia/Adelaide", "Australia/Brisbane",
    "Australia/Broken_Hill", "Australia/Canberra", "Australia/Currie",
    "Australia/Darwin", "Australia/Eucla", "Australia/Hobart",
    "Australia/LHI", "Australia/Lindeman", "Australia/Lord_Howe",
    "Australia/Melbourne", "Australia/NSW", "Australia/North",
    "Australia/Perth", "Australia/Queensland", "Australia/South",
    "Australia/Sydney", "Australia/Tasmania", "Australia/Victoria",
    "Australia/West", "Australia/Yancowinna",
    "Brazil/Acre", "Brazil/DeNoronha", "Brazil/East", "Brazil/West",
    "CET", "CST6CDT", "Canada/Atlantic", "Canada/Central",
    "Canada/Eastern", "Canada/Mountain", "Canada/Newfoundland",
    "Canada/Pacific", "Canada/Saskatchewan", "Canada/Yukon",
    "Chile/Continental", "Chile/EasterIsland",
    "Cuba", "EET", "EST", "EST5EDT", "Egypt", "Eire",
    "Etc/GMT", "Etc/GMT+0", "Etc/GMT+1", "Etc/GMT+10", "Etc/GMT+11",
    "Etc/GMT+12", "Etc/GMT+2", "Etc/GMT+3", "Etc/GMT+4", "Etc/GMT+5",
    "Etc/GMT+6", "Etc/GMT+7", "Etc/GMT+8", "Etc/GMT+9", "Etc/GMT-0",
    "Etc/GMT-1", "Etc/GMT-10", "Etc/GMT-11", "Etc/GMT-12", "Etc/GMT-13",
    "Etc/GMT-14", "Etc/GMT-2", "Etc/GMT-3", "Etc/GMT-4", "Etc/GMT-5",
    "Etc/GMT-6", "Etc/GMT-7", "Etc/GMT-8", "Etc/GMT-9", "Etc/GMT0",
    "Etc/Greenwich", "Etc/UCT", "Etc/UTC", "Etc/Universal", "Etc/Zulu",
    "Europe/Amsterdam", "Europe/Andorra", "Europe/Astrakhan",
    "Europe/Athens", "Europe/Belfast", "Europe/Belgrade", "Europe/Berlin",
    "Europe/Bratislava", "Europe/Brussels", "Europe/Bucharest",
    "Europe/Budapest", "Europe/Busingen", "Europe/Chisinau",
    "Europe/Copenhagen", "Europe/Dublin", "Europe/Gibraltar",
    "Europe/Guernsey", "Europe/Helsinki", "Europe/Isle_of_Man",
    "Europe/Istanbul", "Europe/Jersey", "Europe/Kaliningrad",
    "Europe/Kiev", "Europe/Kirov", "Europe/Kyiv", "Europe/Lisbon",
    "Europe/Ljubljana", "Europe/London", "Europe/Luxembourg",
    "Europe/Madrid", "Europe/Malta", "Europe/Mariehamn", "Europe/Minsk",
    "Europe/Monaco", "Europe/Moscow", "Europe/Nicosia", "Europe/Oslo",
    "Europe/Paris", "Europe/Podgorica", "Europe/Prague", "Europe/Riga",
    "Europe/Rome", "Europe/Samara", "Europe/San_Marino", "Europe/Sarajevo",
    "Europe/Saratov", "Europe/Simferopol", "Europe/Skopje", "Europe/Sofia",
    "Europe/Stockholm", "Europe/Tallinn", "Europe/Tirane",
    "Europe/Tiraspol", "Europe/Ulyanovsk", "Europe/Uzhgorod",
    "Europe/Vaduz", "Europe/Vatican", "Europe/Vienna", "Europe/Vilnius",
    "Europe/Volgograd", "Europe/Warsaw", "Europe/Zagreb",
    "Europe/Zaporozhye", "Europe/Zurich",
    "GB", "GB-Eire", "GMT", "GMT+0", "GMT-0", "GMT0", "Greenwich",
    "HST", "Hongkong", "Iceland",
    "Indian/Antananarivo", "Indian/Chagos", "Indian/Christmas",
    "Indian/Cocos", "Indian/Comoro", "Indian/Kerguelen", "Indian/Mahe",
    "Indian/Maldives", "Indian/Mauritius", "Indian/Mayotte",
    "Indian/Reunion",
    "Iran", "Israel", "Jamaica", "Japan", "Kwajalein", "Libya",
    "MET", "MST", "MST7MDT", "Mexico/BajaNorte", "Mexico/BajaSur",
    "Mexico/General", "NZ", "NZ-CHAT", "Navajo",
    "Pacific/Apia", "Pacific/Auckland", "Pacific/Bougainville",
    "Pacific/Chatham", "Pacific/Chuuk", "Pacific/Easter", "Pacific/Efate",
    "Pacific/Enderbury", "Pacific/Fakaofo", "Pacific/Fiji",
    "Pacific/Funafuti", "Pacific/Galapagos", "Pacific/Gambier",
    "Pacific/Guadalcanal", "Pacific/Guam", "Pacific/Honolulu",
    "Pacific/Johnston", "Pacific/Kanton", "Pacific/Kiritimati",
    "Pacific/Kosrae", "Pacific/Kwajalein", "Pacific/Majuro",
    "Pacific/Marquesas", "Pacific/Midway", "Pacific/Nauru", "Pacific/Niue",
    "Pacific/Norfolk", "Pacific/Noumea", "Pacific/Pago_Pago",
    "Pacific/Palau", "Pacific/Pitcairn", "Pacific/Pohnpei",
    "Pacific/Ponape", "Pacific/Port_Moresby", "Pacific/Rarotonga",
    "Pacific/Saipan", "Pacific/Samoa", "Pacific/Tahiti", "Pacific/Tarawa",
    "Pacific/Tongatapu", "Pacific/Truk", "Pacific/Wake", "Pacific/Wallis",
    "Pacific/Yap",
    "Poland", "Portugal", "ROC", "ROK", "Singapore", "Turkey",
    "UCT", "US/Alaska", "US/Aleutian", "US/Arizona", "US/Central",
    "US/East-Indiana", "US/Eastern", "US/Hawaii", "US/Indiana-Starke",
    "US/Michigan", "US/Mountain", "US/Pacific", "US/Samoa",
    "UTC", "Universal", "W-SU", "WET", "Zulu",
];

const CAL_IDS: [&str; 17] = [
    "buddhist", "chinese", "coptic", "dangi", "ethioaa", "ethiopic",
    "gregory", "hebrew", "indian", "islamic-civil", "islamic-tbla",
    "islamic-umalqura", "iso8601", "japanese", "julian", "persian", "roc",
];

// Sorted (name, canonical_index) pairs — mirrors TZ_SORTED on storage-redesign
fn build_tz_sorted() -> Vec<(&'static str, u16)> {
    let mut sorted: Vec<(&'static str, u16)> = TZ_CANONICAL
        .iter()
        .enumerate()
        .map(|(i, &name)| (name, u16::try_from(i).unwrap()))
        .collect();
    sorted.sort_unstable_by_key(|&(name, _)| name);
    sorted
}

fn binary_search(sorted: &[(&str, u16)], id: &str) -> Option<u16> {
    sorted
        .binary_search_by_key(&id, |&(name, _)| name)
        .ok()
        .map(|pos| sorted[pos].1)
}

// ---------------------------------------------------------------------------
// Write path
//
// main: store tz_id as an owned String — `String::from(tz_str)`
// storage-redesign equivalent: binary_search → store a u16
// ---------------------------------------------------------------------------

fn bench_write_path(c: &mut Criterion) {
    let sorted = build_tz_sorted();
    let mut group = c.benchmark_group("tz/write path");

    // main: allocate + copy the string into storage
    group.bench_function("main — String::from (UTC, short)", |b| {
        b.iter(|| String::from(black_box("UTC")));
    });
    group.bench_function("main — String::from (ComodRivadavia, long)", |b| {
        b.iter(|| String::from(black_box("America/Argentina/ComodRivadavia")));
    });
    group.bench_function("main — String::from (Europe/London, middle)", |b| {
        b.iter(|| String::from(black_box("Europe/London")));
    });
    group.bench_function("main — String::from (Not/A/Zone, miss/invalid)", |b| {
        b.iter(|| String::from(black_box("Not/A/Zone")));
    });

    // storage-redesign equivalent: binary search → u16
    group.bench_function("redesign — binary_search (UTC, short)", |b| {
        b.iter(|| binary_search(&sorted, black_box("UTC")));
    });
    group.bench_function("redesign — binary_search (ComodRivadavia, long)", |b| {
        b.iter(|| binary_search(&sorted, black_box("America/Argentina/ComodRivadavia")));
    });
    group.bench_function("redesign — binary_search (Europe/London, middle)", |b| {
        b.iter(|| binary_search(&sorted, black_box("Europe/London")));
    });
    group.bench_function("redesign — binary_search (Not/A/Zone, miss)", |b| {
        b.iter(|| binary_search(&sorted, black_box("Not/A/Zone")));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Read path
//
// main: borrow &str from an owned String — `.as_str()` / deref coercion
// storage-redesign equivalent: bounds-checked index into &'static [&str]
// ---------------------------------------------------------------------------

fn bench_read_path(c: &mut Criterion) {
    // Simulate a stored ZonedDateTime field on main: an owned String on the heap.
    let stored_utc = String::from("UTC");
    let stored_long = String::from("America/Argentina/ComodRivadavia");
    let stored_middle = String::from("Europe/London");

    let mut group = c.benchmark_group("tz/read path");

    // main: return &str from stored String (deref to str pointer + length)
    group.bench_function("main — &str from String (UTC)", |b| {
        b.iter(|| black_box(stored_utc.as_str()));
    });
    group.bench_function("main — &str from String (ComodRivadavia)", |b| {
        b.iter(|| black_box(stored_long.as_str()));
    });
    group.bench_function("main — &str from String (Europe/London)", |b| {
        b.iter(|| black_box(stored_middle.as_str()));
    });

    // storage-redesign equivalent: bounds-checked index → &'static str
    group.bench_function("redesign — array index (idx=0 → first)", |b| {
        b.iter(|| TZ_CANONICAL.get(usize::from(black_box(0u16))).copied());
    });
    let last_idx = (TZ_CANONICAL.len() - 1) as u16;
    group.bench_function("redesign — array index (idx=last)", |b| {
        b.iter(|| TZ_CANONICAL.get(usize::from(black_box(last_idx))).copied());
    });
    group.bench_function("redesign — array index (idx=200 → mid)", |b| {
        b.iter(|| TZ_CANONICAL.get(usize::from(black_box(200u16))).copied());
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Calendar write + read paths (same pattern, 17 IDs)
// ---------------------------------------------------------------------------

fn bench_cal_write_path(c: &mut Criterion) {
    let mut cal_sorted: Vec<(&str, u8)> = CAL_IDS
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, u8::try_from(i).unwrap()))
        .collect();
    cal_sorted.sort_unstable_by_key(|&(name, _)| name);

    let mut group = c.benchmark_group("cal/write path");
    for id in CAL_IDS {
        group.bench_with_input(
            criterion::BenchmarkId::new("main — String::from", id),
            id,
            |b, id| b.iter(|| String::from(black_box(id))),
        );
        group.bench_with_input(
            criterion::BenchmarkId::new("redesign — binary_search", id),
            id,
            |b, id| {
                b.iter(|| {
                    cal_sorted
                        .binary_search_by_key(&id, |&(name, _)| name)
                        .ok()
                        .map(|pos| cal_sorted[pos].1)
                })
            },
        );
    }
    group.finish();
}

fn bench_cal_read_path(c: &mut Criterion) {
    let stored: Vec<String> = CAL_IDS.iter().map(|&id| String::from(id)).collect();

    let mut group = c.benchmark_group("cal/read path");
    for (i, id) in CAL_IDS.iter().enumerate() {
        group.bench_with_input(
            criterion::BenchmarkId::new("main — &str from String", id),
            &i,
            |b, &i| b.iter(|| black_box(stored[i].as_str())),
        );
        group.bench_with_input(
            criterion::BenchmarkId::new("redesign — array index", id),
            &i,
            |b, &i| b.iter(|| CAL_IDS.get(black_box(i)).copied()),
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// HashMap comparison baseline (algorithmic alternative to binary search)
// ---------------------------------------------------------------------------

fn bench_tz_hashmap_vs_bsearch(c: &mut Criterion) {
    let sorted = build_tz_sorted();
    let map: HashMap<&'static str, u16> = TZ_CANONICAL
        .iter()
        .enumerate()
        .map(|(i, &name)| (name, u16::try_from(i).unwrap()))
        .collect();

    let mut group = c.benchmark_group("tz/HashMap vs binary_search");

    for (label, tz) in [
        ("UTC (short)", "UTC"),
        ("ComodRivadavia (long)", "America/Argentina/ComodRivadavia"),
        ("Europe/London (middle)", "Europe/London"),
        ("Not/A/Zone (miss)", "Not/A/Zone"),
    ] {
        group.bench_with_input(
            criterion::BenchmarkId::new("HashMap", label),
            tz,
            |b, tz| b.iter(|| map.get(black_box(tz))),
        );
        group.bench_with_input(
            criterion::BenchmarkId::new("binary_search", label),
            tz,
            |b, tz| b.iter(|| binary_search(&sorted, black_box(tz))),

        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_write_path,
    bench_read_path,
    bench_cal_write_path,
    bench_cal_read_path,
    bench_tz_hashmap_vs_bsearch,
);
criterion_main!(benches);
