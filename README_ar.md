# M-ust — TUI للتعرف على الأغاني (Rust 100%)

> يلتقط صوت الجهاز أو تطبيق محدد، ويتعرف على الأغنية عبر Shazam (مجاني، بدون مفتاح).
>
> **الدعم:** Windows ✅ · Linux / macOS ⚠️ غير مختبر — التبليغ مرحب به.
>
> [English](README.md) | العربية

## المميزات
- واجهة نصية (`ratatui`) — اختر المصدر واضغط `r`
- ثلاثة مصادر: مايكروفون، صوت الجهاز (loopback)، أو تطبيق محدد (Windows)
- AcoustID احتياطي اختياري بمفتاح مجاني
- سجل الاستماع (`h`) ووضع الاستماع المستمر (`l` / `--loop-mode`)

لا يُرفع إلا بصمات مجهولة — الصوت الخام لا يغادر جهازك أبداً.

## التثبيت

```powershell
# سطر واحد (Windows) — نسخة جاهزة من آخر GitHub Release:
irm https://raw.githubusercontent.com/A2kliDis/m-ust/master/install.ps1 | iex
```

```bash
# سطر واحد (Linux/macOS):
curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/master/install.sh | bash
```

من المصدر (يتطلب Rust):

```powershell
cargo install --git https://github.com/A2kliDis/m-ust --bin m-ust
cargo install --path . --bin m-ust   # نسخة محلية
```

الحذف (يسألك عن الإعدادات والسجل):

```powershell
irm https://raw.githubusercontent.com/A2kliDis/m-ust/master/uninstall.ps1 | iex
# curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/master/uninstall.sh | bash   # Linux/macOS
```

يُثبت في `%LOCALAPPDATA%\m-ust` (أو `~/.local/bin` على Linux/macOS) ويضيفه للـ PATH.
وضع محمول (Windows، بدون PATH): `$env:M_UST_NO_PATH=1; irm ... | iex`.

## التشغيل

```powershell
m-ust                                 # الافتراضي 12 ثانية
m-ust --duration 12
m-ust --loop-mode                     # استماع مستمر حتى الخروج
m-ust --acoustid-key YOUR_KEY         # تفعيل AcoustID الاحتياطي
$env:M_UST_ACOUSTID_KEY="YOUR_KEY"; m-ust   # نفس الشيء دون حفظ على القرص
```

الأزرار داخل الواجهة: `Tab` تبديل المصدر • `↑/↓` اختيار • `r` تسجيل • `l` مستمر • `h` السجل • `o` فتح الأغنية • `c` مسح • `q` خروج

## إعداد الصوت
- **Windows**: يعمل مباشرة. التقاط تطبيق محدد يحتاج Windows 10 2004+.
- **Linux**: اختر مصدر `Monitor of ...` عبر (`pactl list sources | grep monitor`).
- **macOS**: ثبّت BlackHole 2ch واختره كمدخل.

## AcoustID الاحتياطي (اختياري)
1. مفتاح مجاني من https://acoustid.org/new
2. مرره عبر `--acoustid-key` أو `M_UST_ACOUSTID_KEY`
3. يحتاج `fpcalc` (Chromaprint) بجانب `m-ust.exe` أو في `PATH` — بدونه يعمل وضع Shazam طبيعياً

الإعدادات في `%APPDATA%\m-ust\config.toml` (Windows) أو `~/.config/m-ust/config.toml` (Linux) ولا تُرفع أبداً.
