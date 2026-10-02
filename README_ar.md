# M-ust — TUI للتعرف على الأغاني من صوت الجهاز (Rust 100%)

> بناء كامل بـ Rust • مجاني 100% • يلتقط صوت الجهاز أو تطبيق محدد
>
> [English](README.md) | العربية

## المميزات
- **TUI بـ ratatui** : واجهة نصية سريعة، تختار المصدر وتضغط `r`
- **ثلاث أوضاع التقاط**:
  1. 🎤 مايكروفون (CPAL)
  2. 🔊 صوت الجهاز Loopback — كل ما يخرج من السماعات
  3. 🎯 تطبيق محدد (Windows Process Loopback)
- **بصمة Shazam نقية Rust** : `rustfft` + خوارزمية Wang 2003 (مثل SongRec) — بدون مكتبات C
- **مجاني 100%** : يرسل البصمة فقط (peaks) إلى `amp.shazam.com` — لا يحتاج مفتاح، لا يرسل صوت خام
- **Fallback AcoustID** : اختياري إذا فشل Shazam، بمفتاح مجاني من `acoustid.org`

## كيف يعمل؟

```
[ CPAL / WASAPI Loopback ] -> mono 16kHz (downmix+resample)
        ↓
[ Shazam Fingerprint ] 2048 FFT + Hanning -> Peak Spreading (freq/time) -> 4 bands
        ↓
[ POST https://amp.shazam.com/discovery/v5/... ] -> JSON {title, artist, album, url}
```

البصمة = قائمة قمم ترددية `(freq, time)` فقط، لا يمكن إعادة بناء الصوت منها. الخصوصية محفوظة.

## التثبيت

```powershell
# سطر واحد (Windows) — نسخة جاهزة من آخر GitHub Release،
# ويرجع لـ cargo إذا لا يوجد إصدار بعد:
irm https://raw.githubusercontent.com/A2kliDis/m-ust/main/install.ps1 | iex
```

```bash
# سطر واحد (Linux/macOS):
curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/main/install.sh | bash
```

من المصدر (يتطلب Rust):

```powershell
cargo install --git https://github.com/A2kliDis/m-ust --bin m-ust

# أو محلياً
cargo install --path . --bin m-ust
```

## التشغيل

```powershell
m-ust                    # مدة افتراضية 12 ثانية
m-ust --duration 12
m-ust --acoustid-key YOUR_KEY --duration 15
# أو عبر متغير البيئة (لا يُحفظ في ملف الإعدادات):
$env:M_UST_ACOUSTID_KEY="YOUR_KEY"; m-ust
```

للتطوير:

```powershell
cargo run -- --duration 12
cargo run -- --acoustid-key YOUR_KEY --duration 15
```

داخل TUI:
- `Tab` : تبديل المصدر (مايكروفون / جهاز / تطبيق)
- `↑/↓` : اختيار جهاز/تطبيق
- `r` : تسجيل وتعرف
- `o` : فتح صفحة الأغنية في المتصفح (صورة الغلاف تظهر داخل الواجهة)
- `l` : وضع الاستماع المستمر Loop ON/OFF (أو `m-ust --loop-mode`) — يتخطى تكرار نفس الأغنية المتتالية تلقائياً
- `h` : عرض آخر 5 من السجل (يُحفظ تلقائياً في `%APPDATA%\m-ust\history.csv`)
- `c` : مسح السجل
- `q` : خروج

## التقاط صوت الجهاز — حسب النظام

### Windows (مُختبر)
- **System Loopback**: مدمج عبر WASAPI loopback — يلتقط بالضبط ما تسمعه في السماعات. إذا فشل، جرّب جهاز إخراج آخر، أو فعّل `Stereo Mix` من إعدادات الصوت كحل بديل.
- **Per-App**: عبر Windows Process Loopback (يتطلب Windows 10 2004+). اختر `chrome.exe` / `spotify.exe` من القائمة، والكود يلتقط تلك العملية فقط (PID حقيقي عبر `IAudioSessionManager2`).

### Linux
- **System Loopback**: ابحث عن جهاز `Monitor of ...` (PulseAudio/PipeWire). الكود يبحث تلقائياً عن أي input يحتوي `monitor`.
  ```bash
  pactl list sources | grep monitor
  # ثم اختره من TUI
  ```
  إذا كان لديك PipeWire + Pulse معاً قد ترى `no node available` — احذف `pulseaudio` وثبّت `pipewire-pulse`.

### macOS
- لا يوجد loopback افتراضي. ثبّت **BlackHole 2ch**:
  ```bash
  brew install blackhole-2ch
  ```
  ثم أنشئ Multi-Output Device في `Audio MIDI Setup` واختره كـ output، و `BlackHole` كـ input في TUI.

## مجاني 100% — ماذا استخدمنا؟

| المكون | الترخيص | التكلفة |
|--------|---------|---------|
| `ratatui`, `crossterm`, `cpal`, `rustfft`, `reqwest`, `hound` | MIT/Apache2 | مجاني |
| `wasapi` (Windows) | MIT | مجاني |
| Shazam `amp.shazam.com` (غير رسمي، مستخدم في SongRec) | مجاني بدون مفتاح | مجاني |
| AcoustID `api.acoustid.org` | مجاني بمفتاح مجاني | مجاني (3 req/s) |
| MusicBrainz | مجاني | مجاني |

**لا AudD، لا ACRCloud، لا مفاتيح مدفوعة.**

للحصول على مفتاح AcoustID المجاني (اختياري):
1. ادخل https://acoustid.org/new
2. سجل واحصل على `Client API Key`
3. شغّل `m-ust --acoustid-key KEY` أو احفظه في متغير البيئة `M_UST_ACOUSTID_KEY`

> ملاحظة: مسار AcoustID احتياطي فقط ويتطلب `fpcalc` (من Chromaprint).
> بدونه الأداة تعمل طبيعياً عبر Shazam. لا ترفع `fpcalc.exe` للريبو —
> ضعه بجانب `m-ust.exe` أو في `PATH`.
> ملف الإعدادات (`%APPDATA%\m-ust\config.toml`) لا يُرفع أبداً (في `.gitignore`).

## هيكل المشروع

```
src/
  main.rs              # CLI + تشغيل TUI
  config.rs            # AppConfig (env > file)
  history.rs           # حفظ/تحميل history.csv
  audio/
    capture.rs         # CPAL + WASAPI loopback + process loopback + resample 16k
    devices.rs         # سرد الأجهزة وجلسات التطبيقات الحقيقية
  fingerprint/
    shazam.rs          # SignatureGenerator (Port من SongRec)
  api/
    shazam.rs          # POST إلى Shazam
    acoustid.rs        # fallback عبر fpcalc
  tui/
    app.rs             # حالة التطبيق والـ loop والاستماع المستمر
    ui.rs              # رسم ratatui
```

## تطوير إضافي مقترح
- [x] تفعيل WASAPI loopback الحقيقي (`src/audio/capture.rs`)
- [x] سرد PID الحقيقي عبر `IAudioSessionManager2` (بدون أسماء وهمية)
- [x] حفظ السجل في `history.csv` (يُعرض بزر `h`)
- [x] `fpcalc` integration للـ AcoustID الحقيقي (اختياري، يتطلب `chromaprint`)
- [x] وضع Continuous listening (`l` أو `m-ust --loop-mode`)
- [ ] تنبيه التحديثات من داخل الأداة (فحص GitHub Releases)

## بناء release

```powershell
cargo build --release
.\target\release\m-ust.exe
```

> تم بناؤه بـ Rust 1.98، يعمل على Windows/Linux/macOS.
