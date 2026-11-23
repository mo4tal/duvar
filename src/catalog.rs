//! Curated, metadata-based topic catalogue. Never infers image safety from pixels.
use crate::model::{Rating, Wallpaper};
#[derive(Clone, Debug)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub match_keys: Vec<String>,
    pub sensitive: bool,
    pub group: usize,
}
pub const GENERAL_GROUPS: [&str; 10] = [
    "Doğa",
    "Yeryüzü",
    "Mevsimler ve gökyüzü",
    "Uzay",
    "Canlılar",
    "Şehir ve mimari",
    "Ulaşım ve teknoloji",
    "Soyut ve tasarım",
    "Sanat",
    "Kültür ve yaşam",
];
pub const SENSITIVE_GROUPS: [&str; 10] = [
    "Portre ve kompozisyon",
    "İç giyim",
    "Kıyafet",
    "Renk ve malzeme",
    "Poz",
    "Kadraj",
    "Mekân",
    "Işık",
    "Sanat biçimi",
    "Saç",
];
const GENERAL: &str = r#"Manzara|landscape,landscapes,scenery
Dağlar|mountain,mountains,alps
Ormanlar|forest,forests,woodland
Göller|lake,lakes
Nehirler|river,rivers,stream
Şelaleler|waterfall,waterfalls
Denizler|sea,ocean,seascape
Plajlar|beach,beaches,coast
Adalar|island,islands
Çöller|desert,dunes
Kanyonlar|canyon,canyons
Mağaralar|cave,caves
Volkanlar|volcano,volcanoes
Buzullar|glacier,glaciers,iceberg
Kar manzaraları|snow,snowy,winter
Yağmurlu manzaralar|rain,rainy
Sisli manzaralar|fog,mist,foggy
Tropik doğa|tropical,jungle,rainforest
Çayırlar|meadow,grassland,grass
Tarlalar|field,fields,farmland
İlkbahar|spring,blossom
Yaz|summer
Sonbahar|autumn,fall foliage
Kış|winter,frost
Gün doğumu|sunrise,dawn
Gün batımı|sunset,dusk
Gece|night,nightscape
Yıldızlı gökyüzü|starry sky,night sky
Bulutlar|cloud,clouds,cloudscape
Fırtınalar|storm,lightning,thunderstorm
Uzay|space,astronomy
Bulutsular|nebula,nebulae
Galaksiler|galaxy,galaxies
Gezegenler|planet,planets
Ay|moon,lunar
Güneş|sun,solar
Dünya|earth,earth observation
Samanyolu|milky way
Uzay araçları|spacecraft,spaceship,rocket
Aurora|aurora,northern lights
Kuşlar|bird,birds
Kedigiller|cat,cats,feline
Köpekler|dog,dogs,canine
Yaban hayatı|wildlife,wild animals
Deniz canlıları|marine life,fish,underwater
Böcekler|insect,insects,butterfly
Çiçekler|flower,flowers,floral
Ağaçlar|tree,trees
Botanik|plant,plants,botanical
Makro fotoğraf|macro,close up
Şehir silüeti|cityscape,skyline
Gökdelenler|skyscraper,skyscrapers
Tarihi şehirler|old town,historic city
Mimari|architecture,building,buildings
Kaleler|castle,castles
Köprüler|bridge,bridges
Sokaklar|street,streets
İç mekânlar|interior,interiors
Tapınaklar|temple,temples
Endüstriyel yapılar|industrial,factory
Otomobiller|car,cars,automobile
Motosikletler|motorcycle,motorbike
Trenler|train,railway,railroad
Uçaklar|aircraft,airplane,aviation
Gemiler|ship,ships,sailboat
Bisikletler|bicycle,cycling
Uzay teknolojisi|space technology,satellite
Bilgisayarlar|computer,computers
Robotlar|robot,robots
Mekanik ayrıntılar|mechanical,machinery
Soyut|abstract,abstraction
Minimalizm|minimalism,minimalist,minimal
Geometri|geometric,geometry
Fraktallar|fractal,fractals
Dokular|texture,textures
Desenler|pattern,patterns
Tipografi|typography,lettering
Siyah beyaz|black and white,monochrome
Neon|neon,cyberpunk
Koyu palet|dark,low key
Yağlı boya|oil painting,oil on canvas
Suluboya|watercolor,watercolour
Empresyonizm|impressionism,impressionist
Japon baskıları|ukiyo e,japanese print,woodblock
Dijital sanat|digital art,digital painting
İllüstrasyon|illustration,illustrations
Fantastik sanat|fantasy,fantasy art
Bilimkurgu|science fiction,sci fi
Anime manzaraları|anime scenery,anime landscape
Piksel sanatı|pixel art
Sürrealizm|surrealism,surreal
Retro|retro,vintage
Sinema|cinema,movie,film
Oyunlar|game,gaming,video game
Müzik|music,instrument
Portreler|portrait,portraits
Natürmort|still life
Yiyecekler|food,fruit
Spor|sport,sports
Sualtı manzarası|underwater,coral reef"#;
const SENSITIVE: &str = r#"Sanatsal nü|artistic nude,nude art,nude
Yetişkin kadın portresi|mature female,adult woman
Yetişkin erkek portresi|mature male,adult man
Glamour|glamour,glamor
Boudoir|boudoir
Yetişkin çift portresi|couple,adult couple
Tek kişi|solo
İki kişi|duo,2girls,2boys
Grup kompozisyonu|group,multiple girls,multiple boys
Olgun yetişkinler|mature,mature female,mature male
İç çamaşırı|lingerie,underwear
Dantelli kıyafet|lace,lace trim
Korse|corset,corset lingerie
Jartiyer|garter,garter belt
Çoraplar|stockings,thighhighs
File çorap|fishnet stockings,fishnets
Bodysuit|bodysuit
Babydoll|babydoll,negligee
Saten gecelik|nightgown,satin
İpek kıyafet|silk,silk robe
Bikini|bikini
Tek parça mayo|one piece swimsuit,swimsuit
Plaj kıyafeti|beachwear,sarong
Spor sütyeni|sports bra
Crop üst|crop top
Açık omuz|off shoulder,bare shoulders
Sırtı açık kıyafet|backless outfit,backless dress
Dekolteli kıyafet|cleavage,low cut
Transparan kıyafet|see through,transparent clothing
Bol gömlek|oversized shirt,open shirt
Siyah kıyafet|black lingerie,black underwear,black dress
Beyaz kıyafet|white lingerie,white underwear,white dress
Kırmızı kıyafet|red lingerie,red underwear,red dress
Mor kıyafet|purple lingerie,purple underwear
Mavi kıyafet|blue lingerie,blue underwear
Pembe kıyafet|pink lingerie,pink underwear
Deri kıyafet|leather,leather outfit
Lateks kıyafet|latex,latex bodysuit
Kadife kıyafet|velvet
Islak kıyafet|wet clothes,wet shirt
Ayakta poz|standing
Oturarak poz|sitting
Uzanarak poz|lying,reclining
Yandan görünüm|profile,from side
Arkadan görünüm|from behind,back view
Önden görünüm|front view,from front
Omuz üzerinden bakış|looking back,over shoulder
Kameraya bakış|looking at viewer
Gözleri kapalı|closed eyes
Dans pozu|dancing,dance
Tam boy|full body
Belden yukarı|upper body
Yakın portre|close up,portrait
Dikey kompozisyon|portrait orientation,vertical
Yatay kompozisyon|landscape orientation,horizontal
Simetrik kompozisyon|symmetry,symmetrical
Silüet|silhouette
Yansıma|reflection,mirror
Detay kompozisyonu|detail,closeup
Geniş kadraj|wide shot
Stüdyo|studio,studio lighting
Yatak odası|bedroom
Pencere kenarı|window,by window
Balkon|balcony
Terasta|terrace,rooftop
Bahçe|garden
Plaj ortamı|beach
Havuz|pool,swimming pool
Hamam ve spa|bath,spa
Duş ortamı|shower
Doğal ışık|natural light,daylight
Gün batımı ışığı|sunset,golden hour
Loş ışık|dim lighting,low light
Mum ışığı|candlelight,candle
Neon ışık|neon,neon lights
Karşı ışık|backlighting,backlit
Yumuşak ışık|soft lighting,soft light
Sert ışık|hard lighting,dramatic lighting
Renkli ışık|colored lighting,colored light
Siyah beyaz çekim|monochrome,black and white
Fotoğraf|photography,photo,realistic
Dijital resim|digital art,digital painting
Anime çizimi|anime,anime style
Yağlı boya stili|oil painting,oil on canvas
Suluboya stili|watercolor,watercolour
Eskiz|sketch,pencil drawing
Çizgi sanatı|lineart,line art
3B render|3d,3d render
Retro pin-up|pinup,pin up
Gotik stil|gothic,goth
Uzun saç|long hair
Kısa saç|short hair
Kıvırcık saç|curly hair
Örgülü saç|braid,braided hair
At kuyruğu|ponytail
Sarı saç|blonde hair
Siyah saç|black hair
Kızıl saç|red hair
Kahverengi saç|brown hair
Beyaz ve gümüş saç|white hair,silver hair"#;

fn build(sensitive: bool) -> Vec<Topic> {
    (if sensitive { SENSITIVE } else { GENERAL })
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let (title, aliases) = line.split_once('|').expect("built-in catalogue");
            Topic {
                id: format!("{}{:03}", if sensitive { "s" } else { "g" }, i),
                title: title.into(),
                aliases: aliases.split(',').map(str::to_owned).collect(),
                match_keys: aliases
                    .split(',')
                    .map(|a| format!(" {} ", words(a)))
                    .collect(),
                sensitive,
                group: i / 10,
            }
        })
        .collect()
}
pub fn all(sensitive: bool) -> &'static [Topic] {
    static GENERAL: std::sync::OnceLock<Vec<Topic>> = std::sync::OnceLock::new();
    static SENSITIVE: std::sync::OnceLock<Vec<Topic>> = std::sync::OnceLock::new();
    if sensitive {
        SENSITIVE.get_or_init(|| build(true))
    } else {
        GENERAL.get_or_init(|| build(false))
    }
}
fn words(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn matched(item: &Wallpaper) -> impl Iterator<Item = &'static Topic> {
    let mut text = item.title.clone();
    for tag in item.tags.as_deref().unwrap_or(&[]) {
        text.push(' ');
        text.push_str(tag);
    }
    let text = format!(" {} ", words(&text));
    all(matches!(item.rating, Rating::Adult | Rating::Sketchy))
        .iter()
        .filter(move |t| t.match_keys.iter().any(|a| text.contains(a)))
}

pub fn accepts(general: &[String], sensitive: &[String], item: &Wallpaper) -> bool {
    let adult = matches!(item.rating, Rating::Sketchy | Rating::Adult);
    let selected = if adult { sensitive } else { general };
    if selected.is_empty() {
        return true;
    }
    let mut text = item.title.clone();
    for tag in item.tags.as_deref().unwrap_or(&[]) {
        text.push(' ');
        text.push_str(tag);
    }
    let text = format!(" {} ", words(&text));
    all(adult)
        .iter()
        .filter(|t| selected.contains(&t.id))
        .any(|t| t.match_keys.iter().any(|a| text.contains(a)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_has_two_hundred_unique_topics() {
        let all = all(false).iter().chain(all(true)).collect::<Vec<_>>();
        assert_eq!(all.len(), 200);
        let ids = all
            .iter()
            .map(|t| &t.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), 200);
        assert!(
            all.iter()
                .all(|t| !t.aliases.is_empty() && !t.title.is_empty())
        );
    }
}

#[cfg(test)]
mod matching_tests {
    use super::*;
    #[test]
    fn selected_topics_match_metadata_without_enabling_sensitive_content() {
        let item:Wallpaper=serde_json::from_value(serde_json::json!({"id":"fixture","provider":"fixture","title":"Alpine mountain","thumbnail":"","image":"","landing":"","rating":"safe","tags":["mountain"],"width":3840,"height":2160,"author":"","license":""})).unwrap();
        let mut policy = crate::model::Policy {
            general_topics: vec!["g001".into()],
            ..Default::default()
        };
        assert!(policy.accepts(&item));
        policy.general_topics = vec!["g005".into()];
        assert!(!policy.accepts(&item));
        let mut adult = item;
        adult.rating = Rating::Adult;
        adult.tags = Some(vec!["nude".into()]);
        policy.sensitive_topics = vec!["s000".into()];
        assert!(!policy.accepts(&adult));
        policy.adult = true;
        assert!(policy.accepts(&adult));
    }
}

pub fn search_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| *c != '\u{0307}')
        .map(|c| match c {
            'ı' | 'İ' => 'i',
            'ğ' => 'g',
            'ü' => 'u',
            'ş' => 's',
            'ö' => 'o',
            'ç' => 'c',
            '_' => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
#[cfg(test)]
mod normalization_tests {
    #[test]
    fn turkish_search_has_stable_casefold() {
        assert_eq!(super::search_key("İSTANBUL"), super::search_key("istanbul"));
        assert_eq!(super::search_key("DAĞLAR"), super::search_key("Dağlar"));
    }
}
