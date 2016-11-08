//! Regular expressions to help the functions in the text module.

//The regexen included in this module are constructed from smaller composible pieces, partly so
//they can be descriptively named, partly so they can be reused. The decomposition structure is
//copied verbatim from the Objective-C implementation of twitter-text:
//
//https://github.com/twitter/twitter-text/tree/master/objc
//
//However, because rust lacks a #define-like feature and because concat!() only works on string
//literals, the regex templates are constructed as macros, which provides similar semantics at the
//expense of extra used lines and possibly extra compile time. Sorry about that.

use regex::{Regex, RegexBuilder};

///Character class substring denoting characters belonging to no valid group.
macro_rules! invalid_characters {
    () => { "\u{FFFE}\u{FEFF}\u{FFFF}\u{202A}-\u{202E}" };
}
///Character class substring containing ASCII control characters.
macro_rules! ctrl_chars {
    () => { r"\x00-\x1F\x7F"};
}
///Character class substring containing Unicode space characters. (Not the same as
///`\p{White_Space}`, since it include ZWJ and some extra non-WS-tagged characters too.)
macro_rules! unicode_spaces {
    () => { "\u{09}-\u{0D}\u{20}\u{85}\u{A0}\u{1680}\u{180E}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}" };
}
///Character class substring containing accented Latin characters. Meant to be combined with ASCII
///letters in a character class.
macro_rules! latin_accents {
    () => {
        concat!("\u{00C0}-\u{00D6}\u{00D8}-\u{00F6}\u{00F8}-\u{00FF}\u{0100}-\u{024F}\u{0253}-\u{0254}",
                "\u{0256}-\u{0257}\u{0259}\u{025b}\u{0263}\u{0268}\u{026F}\u{0272}\u{0289}\u{02BB}\u{1E00}-\u{1EFF}")
    };
}

///Character class substring denoting punctuation characters.
macro_rules! punctuation_chars {
    () => { r##"-_!"#$%&'()*+,./:;<=>?@\[\]^`{|}~"## };
}
///Character class substring denoting punctuation characters, but not the hyphen (`-`).
macro_rules! punctuation_chars_no_hyphen {
    () => { r##"_!"#$%&'()*+,./:;<=>?@\[\]^`{|}~"## };
}
///Character class substring denoting punctuation characters, but not the hyphen (`-`) or the
///underscore (`_`).
macro_rules! punctuation_chars_no_hyphen_underscore {
    () => { r##"!"#$%&'()*+,./:;<=>?@\[\]^`{|}~"## };
}

///Regex substring containing a character class that can occur before a URL.
macro_rules! valid_url_preceding_chars {
    () => { concat!(r"(?:[^a-zA-Z0-9@＠$#＃", invalid_characters!(), r"]|^)") };
}
///Regex substring containing a character class that can occur at the beginning or end of a URL.
macro_rules! valid_domain_start_end_chars {
    () => { concat!("[^", punctuation_chars!(), ctrl_chars!(), invalid_characters!(), unicode_spaces!(), "]") };
}
///Regex substring containing a character class that can occur in the middle of a subdomain.
macro_rules! valid_subdomain_middle_chars {
    () => { concat!("[^", punctuation_chars_no_hyphen_underscore!(), ctrl_chars!(), invalid_characters!(), unicode_spaces!(), "]") };
}
///Regex substring containing a character class that can occur in the middle of a domain name.
macro_rules! valid_domain_middle_chars {
    () => { concat!("[^", punctuation_chars_no_hyphen!(), ctrl_chars!(), invalid_characters!(), unicode_spaces!(), "]") };
}

//I'm sorry.

///Regex substring containing a pattern that matches a general top-level domain.
macro_rules! valid_gtld {
    () => {
        concat!("(?:",
                    "삼성|닷컴|닷넷|香格里拉|餐厅|食品|飞利浦|電訊盈科|集团|通販|购物|谷歌|诺基亚|联通|网络|网站|网店|网址|组织机构|移动|珠宝|点看|游戏|淡马锡|机构|書籍|时尚|新闻|政府|政务|",
                    "手表|手机|我爱你|慈善|微博|广东|工行|家電|娱乐|大拿|大众汽车|在线|嘉里大酒店|嘉里|商标|商店|商城|公益|公司|八卦|健康|信息|佛山|企业|中文网|中信|世界|ポイント|ファッション|",
                    "セール|ストア|コム|グーグル|クラウド|みんな|คอม|संगठन|नेट|कॉम|همراه|موقع|موبايلي|كوم|شبكة|بيتك|بازار|العليان|ارامكو|",
                    "ابوظبي|קום|сайт|рус|орг|онлайн|москва|ком|дети|zuerich|zone|zippo|zip|zero|zara|zappos|yun|youtube|",
                    "you|yokohama|yoga|yodobashi|yandex|yamaxun|yahoo|yachts|xyz|xxx|xperia|xin|xihuan|xfinity|xerox|xbox|",
                    "wtf|wtc|wow|world|works|work|woodside|wolterskluwer|wme|winners|wine|windows|win|williamhill|wiki|",
                    "wien|whoswho|weir|weibo|wedding|wed|website|weber|webcam|weatherchannel|weather|watches|watch|warman|",
                    "wanggou|wang|walter|walmart|wales|vuelos|voyage|voto|voting|vote|volvo|volkswagen|vodka|vlaanderen|",
                    "vivo|viva|vistaprint|vista|vision|visa|virgin|vip|vin|villas|viking|vig|video|viajes|vet|",
                    "versicherung|vermögensberatung|vermögensberater|verisign|ventures|vegas|vanguard|vana|vacations|ups|",
                    "uol|uno|university|unicom|uconnect|ubs|ubank|tvs|tushu|tunes|tui|tube|trv|trust|travelersinsurance|",
                    "travelers|travelchannel|travel|training|trading|trade|toys|toyota|town|tours|total|toshiba|toray|top|",
                    "tools|tokyo|today|tmall|tkmaxx|tjx|tjmaxx|tirol|tires|tips|tiffany|tienda|tickets|tiaa|theatre|",
                    "theater|thd|teva|tennis|temasek|telefonica|telecity|tel|technology|tech|team|tdk|tci|taxi|tax|tattoo|",
                    "tatar|tatamotors|target|taobao|talk|taipei|tab|systems|symantec|sydney|swiss|swiftcover|swatch|",
                    "suzuki|surgery|surf|support|supply|supplies|sucks|style|study|studio|stream|store|storage|stockholm|",
                    "stcgroup|stc|statoil|statefarm|statebank|starhub|star|staples|stada|srt|srl|spreadbetting|spot|",
                    "spiegel|space|soy|sony|song|solutions|solar|sohu|software|softbank|social|soccer|sncf|smile|smart|",
                    "sling|skype|sky|skin|ski|site|singles|sina|silk|shriram|showtime|show|shouji|shopping|shop|shoes|",
                    "shiksha|shia|shell|shaw|sharp|shangrila|sfr|sexy|sex|sew|seven|ses|services|sener|select|seek|",
                    "security|secure|seat|scot|scor|scjohnson|science|schwarz|schule|school|scholarships|schmidt|",
                    "schaeffler|scb|sca|sbs|sbi|saxo|save|sas|sarl|sapo|sap|sanofi|sandvikcoromant|sandvik|samsung|",
                    "samsclub|salon|sale|sakura|safety|safe|saarland|ryukyu|rwe|run|ruhr|rsvp|room|rogers|rodeo|rocks|",
                    "rocher|rip|rio|rightathome|ricoh|richardli|rich|rexroth|reviews|review|restaurant|rest|republican|",
                    "report|repair|rentals|rent|ren|reit|reisen|reise|rehab|redumbrella|redstone|red|recipes|realty|",
                    "realtor|realestate|read|raid|radio|racing|qvc|quest|quebec|qpon|pwc|pub|prudential|pru|protection|",
                    "property|properties|promo|progressive|prof|productions|prod|pro|prime|press|praxi|pramerica|post|",
                    "porn|politie|poker|pohl|pnc|plus|plumbing|playstation|play|place|pizza|pioneer|pink|ping|pin|pid|",
                    "pictures|pictet|pics|piaget|physio|photos|photography|photo|philips|pharmacy|pfizer|pet|pccw|pay|",
                    "passagens|party|parts|partners|pars|paris|panerai|panasonic|pamperedchef|page|ovh|ott|otsuka|osaka|",
                    "origins|orientexpress|organic|org|orange|oracle|open|ooo|onyourside|online|onl|ong|one|omega|ollo|",
                    "oldnavy|olayangroup|olayan|okinawa|office|off|observer|obi|nyc|ntt|nrw|nra|nowtv|nowruz|now|norton|",
                    "northwesternmutual|nokia|nissay|nissan|ninja|nikon|nike|nico|nhk|ngo|nfl|nexus|nextdirect|next|news|",
                    "new|neustar|network|netflix|netbank|net|nec|nba|navy|natura|nationwide|name|nagoya|nadex|nab|",
                    "mutuelle|mutual|museum|mtr|mtpc|mtn|msd|movistar|movie|mov|motorcycles|moscow|mortgage|mormon|mopar|",
                    "montblanc|monster|money|monash|mom|moi|moe|moda|mobily|mobi|mma|mls|mlb|mitsubishi|mit|mint|mini|mil|",
                    "microsoft|miami|metlife|meo|menu|men|memorial|meme|melbourne|meet|media|med|mckinsey|mcdonalds|mcd|",
                    "mba|mattel|maserati|marshalls|marriott|markets|marketing|market|mango|management|man|makeup|maison|",
                    "maif|madrid|macys|luxury|luxe|lupin|lundbeck|ltda|ltd|lplfinancial|lpl|love|lotto|lotte|london|lol|",
                    "loft|locus|locker|loans|loan|lixil|living|live|lipsy|link|linde|lincoln|limo|limited|lilly|like|",
                    "lighting|lifestyle|lifeinsurance|life|lidl|liaison|lgbt|lexus|lego|legal|lefrak|leclerc|lease|lds|",
                    "lawyer|law|latrobe|latino|lat|lasalle|lanxess|landrover|land|lancome|lancia|lancaster|lamer|",
                    "lamborghini|ladbrokes|lacaixa|kyoto|kuokgroup|kred|krd|kpn|kpmg|kosher|komatsu|koeln|kiwi|kitchen|",
                    "kindle|kinder|kim|kia|kfh|kerryproperties|kerrylogistics|kerryhotels|kddi|kaufen|juniper|juegos|jprs|",
                    "jpmorgan|joy|jot|joburg|jobs|jnj|jmp|jll|jlc|jewelry|jetzt|jeep|jcp|jcb|java|jaguar|iwc|itv|itau|",
                    "istanbul|ist|ismaili|iselect|irish|ipiranga|investments|intuit|international|intel|int|insure|",
                    "insurance|institute|ink|ing|info|infiniti|industries|immobilien|immo|imdb|imamat|ikano|iinet|ifm|",
                    "ieee|icu|ice|icbc|ibm|hyundai|hyatt|hughes|htc|hsbc|how|house|hotmail|hoteles|hot|hosting|host|horse|",
                    "honeywell|honda|homesense|homes|homegoods|homedepot|holiday|holdings|hockey|hkt|hiv|hitachi|",
                    "hisamitsu|hiphop|hgtv|hermes|here|helsinki|help|healthcare|health|hdfcbank|hdfc|hbo|haus|hangout|",
                    "hamburg|guru|guitars|guide|guge|gucci|guardian|group|gripe|green|gratis|graphics|grainger|gov|got|",
                    "gop|google|goog|goodyear|goodhands|goo|golf|goldpoint|gold|godaddy|gmx|gmo|gmbh|gmail|globo|global|",
                    "gle|glass|glade|giving|gives|gifts|gift|ggee|george|genting|gent|gea|gdn|gbiz|garden|gap|games|game|",
                    "gallup|gallo|gallery|gal|fyi|futbol|furniture|fund|fujixerox|fujitsu|ftr|frontier|frontdoor|frogans|",
                    "frl|fresenius|fox|foundation|forum|forsale|forex|ford|football|foodnetwork|foo|fly|flsmidth|flowers|",
                    "florist|flir|flights|flickr|fitness|fit|fishing|fish|firmdale|firestone|fire|financial|finance|final|",
                    "film|fido|fidelity|fiat|ferrero|ferrari|feedback|fedex|fast|fashion|farmers|farm|fans|fan|family|",
                    "faith|fairwinds|fail|fage|extraspace|express|exposed|expert|exchange|everbank|events|eus|eurovision|",
                    "esurance|estate|esq|erni|ericsson|equipment|epson|epost|enterprises|engineering|engineer|energy|",
                    "emerck|email|education|edu|edeka|eco|eat|earth|dvr|dvag|durban|dupont|duns|dunlop|duck|dubai|dtv|",
                    "drive|download|dot|doosan|domains|doha|dog|dodge|doctor|docs|dnp|diy|dish|discover|discount|",
                    "directory|direct|digital|diet|diamonds|dhl|dev|design|desi|dentist|dental|democrat|delta|deloitte|",
                    "dell|delivery|degree|deals|dealer|deal|dds|dclk|day|datsun|dating|date|dance|dad|dabur|cyou|cymru|",
                    "cuisinella|csc|cruises|crs|crown|cricket|creditunion|creditcard|credit|courses|coupons|coupon|",
                    "country|corsica|coop|cool|cookingchannel|cooking|contractors|contact|consulting|construction|condos|",
                    "comsec|computer|compare|company|community|commbank|comcast|com|cologne|college|coffee|codes|coach|",
                    "clubmed|club|cloud|clothing|clinique|clinic|click|cleaning|claims|cityeats|city|citic|citi|citadel|",
                    "cisco|circle|cipriani|church|chrysler|chrome|christmas|chloe|chintai|cheap|chat|chase|channel|chanel|",
                    "cfd|cfa|cern|ceo|center|ceb|cbs|cbre|cbn|cba|catering|cat|casino|cash|casa|cartier|cars|careers|",
                    "career|care|cards|caravan|car|capitalone|capital|capetown|canon|cancerresearch|camp|camera|cam|",
                    "calvinklein|call|cal|cafe|cab|bzh|buzz|buy|business|builders|build|bugatti|budapest|brussels|brother|",
                    "broker|broadway|bridgestone|bradesco|boutique|bot|bostik|bosch|boots|booking|book|boo|bond|bom|bofa|",
                    "boehringer|boats|bnpparibas|bnl|bmw|bms|blue|bloomberg|blog|blockbuster|blanco|blackfriday|black|biz|",
                    "bio|bingo|bing|bike|bid|bible|bharti|bet|bestbuy|best|berlin|bentley|beer|beauty|beats|bcn|bcg|bbva|",
                    "bbt|bbc|bayern|bauhaus|basketball|bargains|barefoot|barclays|barclaycard|barcelona|bar|bank|band|",
                    "bananarepublic|banamex|baidu|baby|azure|axa|aws|avianca|autos|auto|author|auspost|audio|audible|audi|",
                    "auction|attorney|athleta|associates|asia|asda|arte|art|arpa|army|archi|aramco|aquarelle|apple|app|",
                    "apartments|anz|anquan|android|analytics|amsterdam|amica|amfam|amex|americanfamily|americanexpress|",
                    "alstom|alsace|ally|allstate|allfinanz|alipay|alibaba|alfaromeo|akdn|airtel|airforce|airbus|aigo|aig|",
                    "agency|agakhan|afl|afamilycompany|aetna|aero|aeg|adult|ads|adac|actor|active|aco|accountants|",
                    "accountant|accenture|academy|abudhabi|abogado|able|abc|abbvie|abbott|abb|abarth|aarp|aaa|onion",
                ")")
    };
}
///Regex substring containing a pattern that matches a country-code top-level domain.
macro_rules! valid_cctld {
    () => {
        concat!("(?:",
                    "한국|香港|澳門|新加坡|台灣|台湾|中國|中国|გე|ไทย|ලංකා|ഭാരതം|ಭಾರತ|భారత్|சிங்கப்பூர்|இலங்கை|இந்தியா|ଭାରତ|ભારત|ਭਾਰਤ|ভাৰত|",
                    "ভারত|বাংলা|भारोत|भारतम्|भारत|ڀارت|پاکستان|مليسيا|مصر|قطر|فلسطين|عمان|عراق|سورية|سودان|تونس|بھارت|",
                    "بارت|ایران|امارات|المغرب|السعودية|الجزائر|الاردن|հայ|қаз|укр|срб|рф|мон|мкд|ею|бел|бг|ελ|zw|zm|za|yt|",
                    "ye|ws|wf|vu|vn|vi|vg|ve|vc|va|uz|uy|us|um|uk|ug|ua|tz|tw|tv|tt|tr|tp|to|tn|tm|tl|tk|tj|th|tg|tf|td|",
                    "tc|sz|sy|sx|sv|su|st|ss|sr|so|sn|sm|sl|sk|sj|si|sh|sg|se|sd|sc|sb|sa|rw|ru|rs|ro|re|qa|py|pw|pt|ps|",
                    "pr|pn|pm|pl|pk|ph|pg|pf|pe|pa|om|nz|nu|nr|np|no|nl|ni|ng|nf|ne|nc|na|mz|my|mx|mw|mv|mu|mt|ms|mr|mq|",
                    "mp|mo|mn|mm|ml|mk|mh|mg|mf|me|md|mc|ma|ly|lv|lu|lt|ls|lr|lk|li|lc|lb|la|kz|ky|kw|kr|kp|kn|km|ki|kh|",
                    "kg|ke|jp|jo|jm|je|it|is|ir|iq|io|in|im|il|ie|id|hu|ht|hr|hn|hm|hk|gy|gw|gu|gt|gs|gr|gq|gp|gn|gm|gl|",
                    "gi|gh|gg|gf|ge|gd|gb|ga|fr|fo|fm|fk|fj|fi|eu|et|es|er|eh|eg|ee|ec|dz|do|dm|dk|dj|de|cz|cy|cx|cw|cv|",
                    "cu|cr|co|cn|cm|cl|ck|ci|ch|cg|cf|cd|cc|ca|bz|by|bw|bv|bt|bs|br|bq|bo|bn|bm|bl|bj|bi|bh|bg|bf|be|bd|",
                    "bb|ba|az|ax|aw|au|at|as|ar|aq|ao|an|am|al|ai|ag|af|ae|ad|ac",
                ")")
    };
}
///Regex substring containing a pattern matching a non-ASCII top-level domain rendered in Punycode.
macro_rules! valid_punycode {
    () => { "(?:xn--[0-9a-z]+)" };
}
///Regex substring containing a pattern matching ccTLDs used in a standalone short URL.
macro_rules! valid_special_cctld {
    () => {
        concat!("(?:",
                    "co|tv",
                ")")
    };
}

///Regex substring containing a character class that can occur in a URL path.
macro_rules! valid_general_url_path_chars {
    () => {
        concat!(r"[-a-zA-Z\p{Cyrillic}0-9!\*';:=+,.$/%#\[\]_~&|@", latin_accents!(), "]")
    };
}
///Regex substring containing a pattern matching characters in a valid URL path with one or two
///sets of matching parentheses.
macro_rules! valid_url_balancing_parens {
    () => {
        concat!(r"\(",
                    "(?:",
                        valid_general_url_path_chars!(), "+",
                    "|",
                        "(?:", r"\(", valid_general_url_path_chars!(), "+", r"\)", ")",
                    ")",
                r"\)")
    };
}
///Regex substring containing a pattern matching valid characters at the end of a URL path.
macro_rules! valid_general_url_path_ending_chars {
    () => {
        concat!(r"[-a-zA-Z\p{Cyrillic}0-9=_#/+", latin_accents!(), "]|(?:", valid_url_balancing_parens!(), ")")
    };
}

///Regex substring containing a pattern matching a subdomain.
macro_rules! valid_subdomain {
    () => {
        concat!("(?:",
                    "(?:", valid_domain_start_end_chars!(), valid_subdomain_middle_chars!(), "*)?",
                    valid_domain_start_end_chars!(), r"\.",
                ")")
    };
}
///Regex substring containing a pattern matching a domain name.
macro_rules! valid_domain_name {
    () => {
        concat!("(?:",
                    "(?:", valid_domain_start_end_chars!(), valid_domain_middle_chars!(), "*)?",
                    valid_domain_start_end_chars!(), r"\.",
                ")")
    };
}
///Regex substring containing a simplified pattern matching TLDs.
macro_rules! simplified_valid_tld {
    () => { concat!(valid_domain_start_end_chars!(), "{2,}") };
}
///Regex substring containing a simplified pattern matching a full domain.
macro_rules! simplified_valid_domain {
    () => {
        concat!("(?:",
                    valid_subdomain!(), "*", valid_domain_name!(), simplified_valid_tld!(),
                ")")
    };
}
///Regex matching a valid ASCII-only domain.
macro_rules! valid_ascii_domain {
    () => {
        concat!("(",
                    r"(?:[-a-zA-Z0-9][a-zA-Z0-9_", latin_accents!(), r"]*\.)+",
                    "(?:", valid_gtld!(), "|", valid_cctld!(), valid_punycode!(), ")",
                ")",
                "(?:[^0-9a-z@]|$)")
    };
}
///Regex matching a domain that cannot appear without a path.
macro_rules! invalid_short_domain {
    () => {
        concat!(r"\A", valid_domain_name!(), valid_cctld!(), r"\z")
    };
}
///Regex matching a domain that is exempt from `invalid_short_domain` exclusion.
macro_rules! valid_special_short_domain {
    () => {
        concat!(r"\A", valid_domain_name!(), valid_special_cctld!(), r"\z")
    };
}
///Regex substring containing a pattern matching a full domain. Will compare the top-level domain
///against a known list of TLDs.
macro_rules! url_domain_for_validation {
    () => {
        concat!(r"\A(?:",
                    valid_subdomain!(), "*", valid_domain_name!(),
                    "(?:", valid_gtld!(), "|", valid_cctld!(), "|", valid_punycode!(), ")",
                r")\z")
    };
}

///Regex substring containing a character class matching valid characters within a URL query
///string.
macro_rules! valid_url_query_chars {
    () => { r"[-a-zA-Z0-9!?*'\(\);:&=+$/%#\[\]_\.,~|@]" };
}
///Regex substring containing a character class matching valid characters for the end of a URL
///query string.
macro_rules! valid_url_query_ending_chars {
    () => { "[a-zA-Z0-9_&=#/]" };
}

///Regex substring containing a pattern matching a valid URL path.
macro_rules! valid_url_path {
    () => {
        concat!("(?:",
                    "(?:",
                        valid_general_url_path_chars!(), "*",
                        "(?:", valid_url_balancing_parens!(), valid_general_url_path_chars!(), "*)*",
                        valid_general_url_path_ending_chars!(),
                    ")",
                "|",
                    "(?:", valid_general_url_path_chars!(), "+/)",
                ")")
    };
}

///Simplified regex matching a complete URL. Does not validate against a known TLD list.
macro_rules! simplified_valid_url {
    () => {
        concat!("(",
                    "(", valid_url_preceding_chars!(), ")",
                    "(",
                        "(https?://)?",
                        "(", simplified_valid_domain!(), ")",
                        "(?::([0-9]+))?",
                        "(/", valid_url_path!(), "*)?",
                        r"(\?", valid_url_query_chars!(), "*", valid_url_query_ending_chars!(), ")?",
                    ")",
                ")")
    };
}

///Regex substring containing a character class matching valid characters that can come before a
///user mention.
macro_rules! valid_mention_preceding_chars {
    () => { "(?:[^a-zA-Z0-9_!#$%&*@＠]|^|(?:^|[^a-zA-Z0-9_+~.-])RT:?)" };
}
///Regex substring containing a character class matching valid at-signs that can be part of a user
///mention.
macro_rules! at_signs {
    () => { "[@＠]" };
}
///Regex matching characters that can occur immediately after a user or list mention.
macro_rules! end_mention_match {
    () => {
        concat!(r"\A(?:", at_signs!(), "|[", latin_accents!(), "]|://)")
    };
}

///Regex matching a valid user or list mention.
macro_rules! valid_mention_or_list {
    () => {
        concat!("(", valid_mention_preceding_chars!(), ")",
                "(", at_signs!(), ")",
                "([a-zA-Z0-9_]{1,20})",
                "(/[a-zA-Z][a-zA-Z0-9_-]{0,24})?")
    };
}

///Character class substring containing special characters that can appear within a hashtag.
macro_rules! hashtag_special_chars {
    () => { "_\u{200c}\u{200d}\u{a67e}\u{05be}\u{05f3}\u{05f4}\u{ff5e}\u{301c}\u{309b}\u{309c}\u{30a0}\u{30fb}\u{3003}\u{0f0b}\u{0f0c}\u{00b7}" };
}
///Regex substring containing a character class matching letters that must appear in a hashtag for
///it to be valid.
macro_rules! hashtag_alpha {
    () => { r"[\p{L}\p{M}]" };
}
///Regex substring containing a character class matching alphanumeric characters allowed in a
///hashtag.
macro_rules! hashtag_alphanumeric {
    () => { concat!(r"[\p{L}\p{M}\p{Nd}", hashtag_special_chars!(), "]") };
}
///Character class substring containing characters that cannot appear at the boundary of a hashtag.
macro_rules! hashtag_boundary_invalid_chars {
    () => { concat!(r"&\p{L}\p{M}\p{Nd}", hashtag_special_chars!()) };
}
///Regex substring matching the beginning or end of a hashtag.
macro_rules! hashtag_boundary {
    () => { concat!("^|$|[^", hashtag_boundary_invalid_chars!(), "]") };
}
///Regex matching characters that are not allowed to be at the beginning of a hashtag.
///
///This regex is not from the original Objective-C implementation; it's included here due to the
///regex crate's lack of lookahead assertions.
macro_rules! hashtag_invalid_initial_chars {
    () => { "\\A[\u{fe0f}\u{20e3}]" };
}

///Regex matching a valid hashtag.
macro_rules! valid_hashtag {
    () => {
        concat!("(?:", hashtag_boundary!(), ")",
                "(",
                    "[#＃]",
                    "(", hashtag_alphanumeric!(), "*", hashtag_alpha!(), hashtag_alphanumeric!(), "*", ")",
                ")")
    };
}
///Regex matching characters that can appear after a hashtag.
macro_rules! end_hashtag_match {
    () => { r"\A(?:[#＃]|://)" };
}

lazy_static! {
    pub static ref RE_SIMPLIFIED_VALID_URL: Regex =
        RegexBuilder::new(simplified_valid_url!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_VALID_TCO_URL: Regex =
        RegexBuilder::new(r"https?://t\.co/[a-zA-Z0-9]+").case_insensitive(true).compile().unwrap();
    pub static ref RE_URL_FOR_VALIDATION: Regex =
        RegexBuilder::new(url_domain_for_validation!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_URL_WO_PROTOCOL_INVALID_PRECEDING_CHARS: Regex =
        Regex::new("[-_./]$").unwrap();
    pub static ref RE_VALID_ASCII_DOMAIN: Regex =
        RegexBuilder::new(valid_ascii_domain!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_INVALID_SHORT_DOMAIN: Regex =
        RegexBuilder::new(invalid_short_domain!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_VALID_SPECIAL_SHORT_DOMAIN: Regex =
        RegexBuilder::new(valid_special_short_domain!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_VALID_MENTION_OR_LIST: Regex =
        RegexBuilder::new(valid_mention_or_list!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_END_MENTION: Regex =
        RegexBuilder::new(end_mention_match!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_VALID_HASHTAG: Regex =
        RegexBuilder::new(valid_hashtag!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_END_HASHTAG: Regex =
        RegexBuilder::new(end_hashtag_match!()).case_insensitive(true).compile().unwrap();
    pub static ref RE_HASHTAG_INVALID_INITIAL_CHARS: Regex =
        Regex::new(hashtag_invalid_initial_chars!()).unwrap();
}
