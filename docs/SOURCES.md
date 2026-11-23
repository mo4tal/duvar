# Duvar — Kaynak Sözleşmeleri / Source Contracts

Duvar yalnızca resmi, belgelenmiş ve herkese açık API’lere bağlanır; belgesiz uç noktaları taklit etmez, erişim kontrollerini atlatmaz ve varsayılan olarak **yalnızca güvenli (SFW)** içerik gösterir.

## Desteklenen 12 Kaynak

| Kaynak | Varsayılan | Arama | Etiket | Tarih | Piksel Boyutu | Derecelendirme | Not |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| **Wallhaven** | Açık | ✓ | ✓ | ✓ | ✓ | SFW / Şüpheli / Yetişkin | Yetişkin sınıfı için isteğe bağlı sistem anahtarlığı API anahtarı |
| **Openverse** | Açık | ✓ | ✓ | — | ✓ | SFW / Hassas | Açık lisans (CC) görsel dizini |
| **NASA Images** | İsteğe bağlı | ✓ | ✓ | ✓ | — | Etiketsiz (SFW varsayılanı yok) | Kamu malı uzay ve bilim görselleri |
| **Bing Daily** | İsteğe bağlı | — | — | ✓ | — | Etiketsiz | Günlük UHD duvar kâğıtları (`wp=true`) |
| **Wikimedia Commons** | İsteğe bağlı | ✓ | ✓ | ✓ | ✓ | Etiketsiz | MediaWiki `imageinfo` API, açık lisans |
| **Yerel Klasör** | İsteğe bağlı | ✓ | — | ✓ | ✓ | Etiketsiz | Kullanıcının seçtiği yerel dizin |
| **Cleveland Museum of Art** | İsteğe bağlı | ✓ | ✓ | — | ✓ | Etiketsiz | CC0 yüksek çözünürlüklü sanat eserleri |
| **The Met** | İsteğe bağlı | ✓ | ✓ | ✓ | — | Etiketsiz | Kamu malı koleksiyon (`v1.1/search`) |
| **Victoria & Albert Museum** | İsteğe bağlı | ✓ | ✓ | — | — | Etiketsiz | IIIF yüksek çözünürlüklü arşiv |
| **Yande.re** | Kapalı | ✓ | ✓ | ✓ | ✓ | SFW / Şüpheli / Yetişkin | Moebooru API; ayrı izinle açılır |
| **AIBooru** | Kapalı | ✓ | ✓ | ✓ | ✓ | SFW / Şüpheli / Yetişkin | Danbooru uyumlu açık API; ayrı izinle açılır |
| **NekosAPI** | Kapalı | ✓ | ✓ | — | — | SFW / Şüpheli / Yetişkin | v4 REST API; ayrı izinle açılır |

> **Neden Unsplash ve Pexels yok?** Her iki platformun mevcut API kullanım koşulları duvar kâğıdı uygulamalarına izin vermediği için bilerek dahil edilmedi.

## Güvenlik ve Gizlilik Kuralları

- **Kapalı başlayan güvenlik politikası (fail-closed):** İlk kurulumda yalnızca `SFW` açıktır. Şüpheli, yetişkin ve etiketsiz içerik izinleri ayrı anahtarlarla kullanıcı tarafından açılır ve açıkça kalıcı kılınmadıkça yalnızca oturum süresince geçerlidir.
- **Reşit olmayan / yasaklı içerik koruması:** Booru kaynaklarından gelen zararlı etiketli kayıtlar istemci tarafında sert kurallarla reddedilir; eksik veya kesilmiş etiket verisi doğrulanmış sayılmaz.
- **Yan dosya (sidecar) yok:** Atıf, lisans ve kaynak üstverileri yalnızca `~/.local/share/duvar/metadata/` içinde tutulur; kullanıcının resim klasörüne JSON yazılmaz.
- **Anahtar güvenliği:** Wallhaven API anahtarı yalnızca mevcut Secret Service anahtarlığında saklanır; ayar, kuyruk veya günlük dosyalarına asla yazılmaz.
- **İstek politikası:** Tüm uzak istekler HTTPS zorunluluğu, 429/5xx hız sınırı bekleme (cooldown) ve açık `Duvar/<sürüm> (+https://github.com/mo4tal/duvar)` User-Agent başlığı ile yapılır.
