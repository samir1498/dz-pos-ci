// This Arabic translation is unreviewed by a native speaker, the same as
// crates/core/src/print/strings.rs's words_ar and every
// fixtures/print/*/ar*.html golden (docs/features.md §4, Printing). It is a
// direct translation of fr.ts, not independently written, and it waits for
// R6 before it goes public (context/plans/20260911-landing-page.md, L2).
import type { Dict } from "./types";

export const ar: Dict = {
  lang: "ar",
  dir: "rtl",
  title: "Dinar POS",
  brandWord: "دينار",
  brandSuffix: "بوس",
  tagline: "الصندوق والفاتورة والمخزون والموردون لمحل جزائري، بلا اشتراك عند الطاولة.",

  hero: {
    heading: "الصندوق يطبع الفاتورة ويحافظ على المخزون محدّثًا.",
    body: "بلا اشتراك شهري وبلا حاجة إلى الإنترنت عند الطاولة: كل شيء يعمل على حاسوب المحل.",
    cta: "تواصل معنا",
  },

  pieces: {
    heading: "ما الذي يقوم به",
    sell: {
      title: "البيع",
      body: "البائع يضيف السلع، ويقبض نقدًا أو بالبطاقة أو بالدين، وتخرج التذكرة من الطاولة خلال ثوانٍ.",
    },
    invoice: {
      title: "الفوترة",
      body: "يطلب الزبون فاتورة؟ زر واحد وقت البيع، فتخرج مرقّمة ضمن سلسلة السنة، ببياناتها الإلزامية واسم الزبون.",
    },
    stock: {
      title: "المخزون",
      body: "كل بيع وكل توريد يحدّثان المخزون، والشاشة الرئيسية تنبّه فور بدء نفاد سلعة.",
    },
    know: {
      title: "المعرفة",
      body: "لوحة القيادة تجيب بنظرة واحدة عن مبيعات اليوم، وما تبقى مستحقًا على الزبائن، وما هو مستحق للموردين.",
    },
  },

  staff: {
    heading: "حساب لكل شخص في المحل",
    intro: "المحل الذي يعمل فيه أكثر من شخص عند الطاولة لا يحتاج أن يرى الجميع نفس الشيء. كل شخص يفتح حسابه الخاص، وما يمكنه فعله يعتمد على دوره: المالك أو المدير أو الكاشير.",
    signIn: "الكاشير يفتح جلسته عند الطاولة برمز من أربعة إلى ستة أرقام؛ البقية، باسم وكلمة مرور.",
    cashierScope: "الكاشير يبيع ويقبض الثمن ويطبع التذكرة، لا أكثر: لا كلفة السلعة على المحل، ولا الهامش عليها، ولا الموردون، ولا المصاريف، ولا صندوق اليوم.",
    cashierLimits: "يمكنه منح تخفيض حتى السقف الذي يحدده المحل؛ وما فوقه يحتاج إلى المالك أو المدير. لا يمكنه أبدًا كتابة سعر آخر على السطر، ولا تجاوز سقف الدين المسموح به لزبون.",
    manager: "المدير يقوم بكل ما يقوم به المالك، باستثناء إدارة الموظفين وقراءة سجل التدقيق.",
    log: "سعر تم تغييره، فاتورة أُلغيت، سقف دين تم تجاوزه: كل ذلك يُسجَّل في السجل باسم من قام به. ومحاولة مرفوضة تُسجَّل فيه أيضًا، ليرى المالك أنها حدثت.",
  },

  fiscal: {
    heading: "ما تطلبه الضريبة، جاهز مسبقًا",
    intro: "ما يسأل عنه صاحب محل جزائري أولًا، قبل كل شيء:",
    // docs/features.md §3, fiscal rules table, row "Numbering": "one
    // uninterrupted chronological series per document kind and per year...
    // numbers never reused".
    numbering: "كل فاتورة تحمل رقمًا ضمن سلسلة السنة، بلا فجوة وبلا تكرار.",
    // docs/features.md §3, fiscal rules table, row "Droit de timbre": "cash
    // only... the whole amount at its band's rate".
    stamp: "الدفع نقدًا يحمل طابع الدمغة، ويُحسب تلقائيًا حسب المبلغ.",
    // docs/features.md §3, fiscal rules table, row "TVA rates": "19 %
    // standard, 9 % reduced, 0 % exempt; rate per product".
    tva: "تُحسب الرسم على القيمة المضافة حسب النسبة، سطرًا بسطر: 19 بالمئة أو 9 بالمئة أو صفر بالمئة حسب المنتج.",
    // docs/features.md §3, fiscal rules table, row "Amount in words":
    // "French, Arabic and English generators, dinars and centimes".
    words: "المبلغ المستحق مكتوب أيضًا بالحروف، بالفرنسية والعربية والإنجليزية.",
    // docs/features.md §3, fiscal rules table, row "Régime fiscal": a dated
    // shop setting, `ifu` or `réel`; under IFU no document names a tax, and
    // every document keeps the regime it was issued under.
    regime: "النظام الجزافي أو النظام الحقيقي: تحت النظام الجزافي لا تذكر أي وثيقة ضريبة، وكل وثيقة تحتفظ بالنظام الذي صدرت تحته.",
  },

  languages: {
    heading: "ثلاث لغات، لا هامش جانبي",
    // docs/features.md, Scope: "Arabic (RTL), French, English on every
    // screen and every printed document, independently selectable (UI
    // language ≠ print language)".
    body: "الشاشة والتذكرة والفاتورة موجودة بالفرنسية والعربية والإنجليزية، ولكل واحد حرية اختيار لغة العمل ولغة الطباعة.",
  },

  screens: {
    heading: "التطبيق في صور",
  },

  audience: {
    heading: "لمن هذا",
    body: "لمحل في الجزائر ما زال يمسك حساباته يدويًا أو على جدول بيانات، ويريد جمع فواتيره ومخزونه وموردّيه في مكان واحد.",
  },

  pricing: {
    heading: "السعر",
    body: "السعر لم يُحدَّد بعد، وما دام كذلك يبقى هذا النموذج مغلقًا. سيُفتح هنا، في هذه الصفحة.",
    formNameLabel: "الاسم",
    formPhoneLabel: "الهاتف",
    formWilayaLabel: "الولاية",
    formSubmit: "طلب التواصل",
    formDisabledNote: "مغلق حتى يُحدَّد السعر.",
  },

  // Arabic here is mine, not a native speaker's (research R6): short
  // factual lines only, to keep the review small when it happens.
  download: {
    heading: "تحميل",
    body: "يُثبَّت دينار على حاسوب الصندوق، بالفرنسية والإنجليزية والعربية. اختر ملف نظامك.",
    windowsLabel: "ويندوز",
    windowsNote: "مثبّت .exe",
    macosLabel: "ماك",
    macosNote: "صورة .dmg",
    linuxLabel: "لينكس",
    linuxNote: "AppImage، اجعله قابلًا للتنفيذ",
    recommended: "لنظامك",
    unsignedNote: "الإصدارات الأولى غير موقّعة: سيُظهر ويندوز تحذيرًا وسيطلب ماك النقر بالزر الأيمن ثم فتح.",
    allLink: "كل الإصدارات",
  },
};
