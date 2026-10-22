# YAMDV V2-B owner test / Sahip testi (prompt 012, A-8)

Unzip `YAMDV-v2b-test.zip` to a normal folder, e.g. `C:\yamdv-test` (not inside the zip view).
`YAMDV-v2b-test.zip` dosyasını normal bir klasöre açın, örn. `C:\yamdv-test` (zip görünümünün içinde değil).
For items 2–12 use `yamdv.exe` from this folder. Nothing in this pack uses the network.
2–12. maddeler için bu klasördeki `yamdv.exe` dosyasını kullanın. Bu paket ağa hiç çıkmaz.

1. **Automatic tests.** Double-click `run-tests.cmd`. It takes about 20–25 minutes; do not touch the mouse or keyboard. Send `test-results.txt` (and the `failures` folder if it is not empty). Optional: `run-tests.cmd --include-system` also checks install/uninstall in a scratch registry key.
   **Otomatik testler.** `run-tests.cmd` dosyasına çift tıklayın. Yaklaşık 20–25 dakika sürer; fareye ve klavyeye dokunmayın. `test-results.txt` dosyasını (`failures` klasörü boş değilse onu da) gönderin. İsteğe bağlı: `run-tests.cmd --include-system` kurma/kaldırmayı geçici bir kayıt defteri anahtarında da dener.
2. **Export colors.** Open `tests\fixtures\colors.md` and follow the steps in it. Export it to PDF and to HTML, in light mode and in dark mode (4 files). Each file must have a white page, no dark areas and no selection or find highlight. Then set Settings ▸ Export ▸ *PDF colors* and *HTML colors* to *Theme* (follow theme): the dark export must be dark everywhere.
   **Dışa aktarma renkleri.** `tests\fixtures\colors.md` dosyasını açıp içindeki adımları izleyin. PDF ve HTML'e açık ve koyu modda aktarın (4 dosya). Her dosyada sayfa beyaz olmalı; koyu alan, seçim ya da arama vurgusu olmamalı. Sonra Ayarlar ▸ Dışa aktarma ▸ *PDF renkleri* ve *HTML renkleri*ni *Tema* (temayı takip et) yapın: koyu moddaki çıktı her yerde koyu olmalı.
3. **PDF export of a large file** (`tests\fixtures\large-1mb.md`, created by step 1). The app stays usable (scroll, menus). The card shows the page, the percentage and the time left, and the taskbar button shows progress. Cancel removes the file. When a run finishes, [Open] and [Show in folder] work.
   **Büyük dosyanın PDF'e aktarılması** (`tests\fixtures\large-1mb.md`, 1. adım oluşturur). Uygulama kullanılabilir kalmalı (kaydırma, menüler). Kartta sayfa, yüzde ve kalan süre, görev çubuğu düğmesinde ilerleme görünmeli. İptal dosyayı silmeli. İş bitince [Aç] ve [Klasörde göster] çalışmalı.
4. **Print.** Print… (Ctrl+P) shows the same card. Closing the window while a job runs asks first.
   **Yazdır.** Yazdır… (Ctrl+P) aynı kartı göstermeli. İş sürerken pencereyi kapatmak önce sormalı.
5. **HTML export of a large file.** The card appears, and the file appears only at the end.
   **Büyük dosyanın HTML'e aktarılması.** Kart görünmeli; dosya yalnızca sonunda oluşmalı.
6. **Settings panel.** Open it in 3 ways: the ⚙ button, Ctrl+, and the ⋯ menu ▸ Settings. Move with `Tab`, the arrow keys and `Esc`. Search for "font".
   **Ayarlar paneli.** 3 yoldan açın: ⚙ düğmesi, Ctrl+, ve ⋯ menüsü ▸ Ayarlar. `Tab`, ok tuşları ve `Esc` ile gezinin. "yazı" / "font" arayın.
7. **Typography.** Change the theme, the body font and the code font (monospace families are listed first). Each change applies at once.
   **Yazı tipi.** Temayı, gövde yazı tipini ve kod yazı tipini değiştirin (eş aralıklı aileler önce listelenir). Her değişiklik anında uygulanmalı.
8. **Colors.** Edit the link color (Appearance, click its swatch), then use *Reset* in the color popover (reset token), *Reset all colors* and the *Use custom colors* switch.
   **Renkler.** Bağlantı rengini değiştirin (Görünüm, renk kutusuna tıklayın); sonra renk penceresindeki *Sıfırla* (tek rengi sıfırla), *Tüm renkleri sıfırla* ve *Özel renkleri kullan* anahtarını deneyin.
9. **Shortcuts.** Rebind *Find*, see the `Ctrl+O` conflict, and apply the Typora preset.
   **Kısayollar.** *Bul* kısayolunu değiştirin, `Ctrl+O` çakışmasını görün ve Typora ön ayarını uygulayın.
10. **Privacy / About.** Use *Clear history* (it asks first), and click a path in About to copy it.
    **Gizlilik / Hakkında.** *Geçmişi temizle*'yi kullanın (önce sorar); Hakkında'da bir yola tıklayıp kopyalayın.
11. **Turkish UI.** Switch the language to Türkçe: the whole panel is in Turkish.
    **Türkçe arayüz.** Dili Türkçe yapın: panelin tamamı Türkçe olmalı.
12. **Open V2-A checks.** Snap Layouts (hover the Maximize button), the dark title bar in dark mode, the ⋯ menu with Alt / F10, the color-scheme cards, F11 full screen.
    **Açık V2-A kontrolleri.** Snap Layouts (Büyüt düğmesinin üzerinde bekleyin), koyu modda koyu başlık çubuğu, Alt / F10 ile ⋯ menüsü, renk şeması kartları, F11 tam ekran.

Please also look at the pictures in `screens\` (settings General light, Appearance dark, color picker, Shortcuts, progress card).
Lütfen `screens\` klasöründeki resimlere de bakın (Genel açık, Görünüm koyu, renk seçici, Kısayollar, ilerleme kartı).
