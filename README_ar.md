# M-ust — TUI للتعرف على الأغاني (Rust 100%)

> يلتقط صوت الجهاز أو تطبيق محدد، ويتعرف على الأغنية عبر Shazam (مجاني، بدون مفتاح).
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
cargo install --path . --bin m-ust   # نسخة محلية
```

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

## القادم
- [ ] تنبيه التحديثات من داخل الأداة (فحص GitHub Releases)
