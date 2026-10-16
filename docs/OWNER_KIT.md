# YAMDV owner kit — Phase 003

The CI runner has a 1024×768 Hyper-V screen without a GPU, so its numbers are trends. This kit
lets you measure and check YAMDV on your own PC. **No Rust and no admin rights needed.**

> **Türkçe sürüm aşağıda.**

## English

### 0. Unpack
1. Download the `owner-kit` artifact from the latest `main` run (Actions → ci → run → Artifacts).
2. Windows marks downloaded ZIPs: right-click the ZIP → **Properties** → tick **Unblock** → OK. Then extract it, e.g. to `C:\yamdv-kit`.
3. The exe files are unsigned, so SmartScreen may warn you. Click **More info → Run anyway**.

Open PowerShell (no admin) in the kit folder:
```powershell
cd C:\yamdv-kit
```

### 1. Open every corpus file
```powershell
Get-ChildItem .\corpus -File | ForEach-Object { Start-Process .\yamdv.exe -ArgumentList "`"$($_.FullName)`"" }
```
For each window, check the following: the title is `<file> — YAMDV`; the text is readable; scrolling (wheel, PgUp/PgDn, Home/End, scrollbar drag) is smooth; selecting with the mouse and pressing Ctrl+C copies text; and Esc or Ctrl+W closes the window.
Open the encoding samples too: `Get-ChildItem .\corpus\encodings\*.md | % { Start-Process .\yamdv.exe -ArgumentList "`"$($_.FullName)`"" }`. Turkish characters must look right.

### 2. Open dialog, drag & drop, multi-file
- `.\yamdv.exe`, then press **Ctrl+O**, multi-select two `.md` files, and click Open. The first file opens in the window and the second in a new window.
- Drag a `.md` file from Explorer onto an empty window. Drag a folder: you should see "Drop Markdown files, not folders".
- `.\yamdv.exe .\corpus\headings.md .\corpus\lists.md` opens two windows.
- **IME:** if you use an IME (Japanese/Chinese/Korean), check that the Open dialog's file-name box accepts IME input.

### 3. Memory (M0 / M1 / M2 / M6)
```powershell
.\xtask.exe memprobe .\yamdv.exe --size 1000x700 --runs 3 --json M0.json --label M0
.\xtask.exe memprobe .\yamdv.exe --doc .\corpus\typical.md --size 1000x700 --runs 3 --json M1.json --label M1
.\xtask.exe memprobe .\yamdv.exe --doc .\corpus\large-1mb.md --size 1000x700 --runs 3 --json M2.json --label M2
.\xtask.exe memprobe .\yamdv.exe --doc .\corpus\typical.md --open-dialog --settle-ms 2000 --size 1000x700 --runs 3 --json M6.json --label M6
```
Send the four JSON files. They include the first-paint time, the client size and your monitor resolution.

### 4. Real scrolling numbers (T4) at full HD
```powershell
.\spike-a.exe --size 1920x1080 --autoscroll --log spike-a-fhd.log
```
Wait until the title shows `SCROLL_DONE`, close the window, and send `spike-a-fhd.log`.

### 5. Light/dark switch
Open `.\yamdv.exe .\corpus\typical-basic.md`, then switch **Settings → Personalization → Colors → Choose your mode** between Light and Dark. The window must follow immediately.

### 6. 150 % display scaling
Set **Settings → System → Display → Scale** to 150 %. Open `typical-basic.md` and `unicode.md`. Text must be sharp, with no blurry bitmap scaling. Also drag a window between monitors with different scales if you have them.

### 7. Phase 003 features (F-15)
Open each file with `.\yamdv.exe .\corpus\<file>` and check:
1. `links.md` — click a `#heading` link: smooth scroll to the heading. Click a relative `.md` link: it opens in the same window; **Alt+←** goes back to the same scroll position, **Alt+→** forward. Click an `http(s)` link: an **Ask** dialog shows the real URL (punycode for IDN hosts) with [Cancel] [Copy link] [Open in browser]. Click the `.exe` link: it is blocked and offers **Show in Explorer**. A `javascript:` link shows a "Blocked link type" toast.
2. `images.md` — every image is a compact chip (▣ + alt text or file name) on a grey rounded background. Remote badges, the UNC/WebDAV paths and the SVG are never fetched or opened. Expected for Phase 003: **local images are not yet decoded** (chips only, see the report §3).
3. `tables.md` — alignment left/center/right, header row, zebra rows, rounded border; select cells and **Ctrl+C**, then paste into Excel: tab-separated.
4. `code.md` — syntax colours per language; hover a code block: the toolbar (language, Copy) appears; Copy puts the code on the clipboard; long lines scroll horizontally (Shift+wheel).
5. `callouts.md` — five GitHub callout colours and icons; the Obsidian `[!info]-` callout is folded, click the chevron to expand.
6. `footnotes.md` — click `[1]`: scroll to the note; click `↩`: back to the reference.
7. `details.md` and `html.md` — `<details>` folds and unfolds on click; centred README header; no script, style or event handler ever runs or shows.
8. **Ctrl+O** — the Open dialog runs in a separate `yamdv.exe --dialog-host` process (Task Manager shows it only while the dialog is open).
9. Select some formatted text, **Ctrl+C**, paste into Word: bold/italic/links kept. **Ctrl+Shift+C** copies Markdown source.

---

## Türkçe

### 0. Açma
1. En son `main` çalıştırmasından `owner-kit` artifact'ını indirin (Actions → ci → çalıştırma → Artifacts).
2. Windows indirilen ZIP'leri işaretler: ZIP'e sağ tıklayın → **Özellikler** → **Engellemeyi kaldır** kutusunu işaretleyin → Tamam. Sonra örneğin `C:\yamdv-kit` klasörüne çıkarın.
3. Exe dosyaları imzasız olduğu için SmartScreen uyarı verebilir. **Ek bilgi → Yine de çalıştır**'a tıklayın.

Kit klasöründe PowerShell açın (yönetici gerekmez):
```powershell
cd C:\yamdv-kit
```

### 1. Tüm örnek dosyaları açın
```powershell
Get-ChildItem .\corpus -File | ForEach-Object { Start-Process .\yamdv.exe -ArgumentList "`"$($_.FullName)`"" }
```
Her pencerede şunları kontrol edin: başlık `<dosya> — YAMDV` olmalı; metin okunaklı olmalı; kaydırma (tekerlek, PgUp/PgDn, Home/End, kaydırma çubuğunu sürükleme) akıcı olmalı; fareyle seçip Ctrl+C metni kopyalamalı; Esc veya Ctrl+W pencereyi kapatmalı.
Kodlama örneklerini de açın: `Get-ChildItem .\corpus\encodings\*.md | % { Start-Process .\yamdv.exe -ArgumentList "`"$($_.FullName)`"" }`. Türkçe karakterler doğru görünmeli.

### 2. Aç penceresi, sürükle-bırak, çoklu dosya
- `.\yamdv.exe` çalıştırıp **Ctrl+O**'ya basın, iki `.md` dosyası seçip Aç'a tıklayın. İlk dosya mevcut pencerede, ikincisi yeni pencerede açılır.
- Gezgin'den bir `.md` dosyasını boş pencereye sürükleyin. Bir klasör sürükleyin: "Klasör değil, Markdown dosyaları bırakın" mesajını görmelisiniz.
- `.\yamdv.exe .\corpus\headings.md .\corpus\lists.md` iki pencere açar.
- **IME:** bir IME (Japonca/Çince/Korece) kullanıyorsanız, Aç penceresindeki dosya adı kutusunun IME girişini kabul ettiğini kontrol edin.

### 3. Bellek (M0 / M1 / M2 / M6)
Yukarıdaki İngilizce bölümdeki dört `xtask.exe memprobe` komutunu aynen çalıştırın ve dört JSON dosyasını gönderin. Dosyalarda ilk çizim süresi, istemci alanı boyutu ve monitör çözünürlüğünüz yer alır.

### 4. Full HD'de gerçek kaydırma ölçümü (T4)
```powershell
.\spike-a.exe --size 1920x1080 --autoscroll --log spike-a-fhd.log
```
Başlıkta `SCROLL_DONE` yazana kadar bekleyin, pencereyi kapatın ve `spike-a-fhd.log` dosyasını gönderin.

### 5. Açık/koyu tema geçişi
`.\yamdv.exe .\corpus\typical-basic.md` açıkken **Ayarlar → Kişiselleştirme → Renkler → Modunuzu seçin** ayarını Açık ve Koyu arasında değiştirin. Pencere hemen uyum sağlamalı.

### 6. %150 ekran ölçeği
**Ayarlar → Sistem → Ekran → Ölçek** değerini %150 yapın. `typical-basic.md` ve `unicode.md` dosyalarını açın. Metin keskin olmalı, bulanık bitmap ölçekleme olmamalı. Farklı ölçekli monitörleriniz varsa bir pencereyi aralarında sürükleyin.

### 7. Faz 003 özellikleri (F-15)
Her dosyayı `.\yamdv.exe .\corpus\<dosya>` ile açın ve kontrol edin:
1. `links.md` — `#başlık` bağlantısına tıklayın: başlığa yumuşak kaydırma. Göreli `.md` bağlantısı aynı pencerede açılır; **Alt+←** aynı kaydırma konumuna geri döner, **Alt+→** ileri. `http(s)` bağlantısı gerçek URL'yi (IDN için punycode) gösteren **Sor** penceresini açar: [İptal] [Bağlantıyı kopyala] [Tarayıcıda aç]. `.exe` bağlantısı engellenir ve **Explorer'da göster** sunar. `javascript:` bağlantısı "Engellenen bağlantı türü" bildirimi gösterir.
2. `images.md` — her görsel gri, yuvarlak köşeli küçük bir etikettir (▣ + alt metin veya dosya adı). Uzak rozetler, UNC/WebDAV yolları ve SVG asla indirilmez veya açılmaz. Faz 003 için beklenen: **yerel görseller henüz çözülmüyor** (yalnızca etiket, rapor §3'e bakın).
3. `tables.md` — sola/ortaya/sağa hizalama, başlık satırı, zebra satırlar, yuvarlak kenarlık; hücreleri seçip **Ctrl+C**, Excel'e yapıştırın: sekmeyle ayrılmış.
4. `code.md` — dile göre renklendirme; kod bloğunun üzerine gelin: araç çubuğu (dil, Kopyala) görünür; uzun satırlar yatay kayar (Shift+tekerlek).
5. `callouts.md` — beş GitHub callout rengi ve simgesi; Obsidian `[!info]-` kapalı gelir, oka tıklayınca açılır.
6. `footnotes.md` — `[1]`'e tıklayın: nota kayar; `↩`: referansa döner.
7. `details.md` ve `html.md` — `<details>` tıklayınca açılır/kapanır; ortalanmış README başlığı; hiçbir script, stil veya olay işleyici çalışmaz ya da görünmez.
8. **Ctrl+O** — Aç penceresi ayrı bir `yamdv.exe --dialog-host` sürecinde çalışır (Görev Yöneticisi'nde yalnızca pencere açıkken görünür).
9. Biçimli metin seçip **Ctrl+C**, Word'e yapıştırın: kalın/italik/bağlantılar korunur. **Ctrl+Shift+C** Markdown kaynağını kopyalar.

