// static FONT_

static FONT_SOURCE_HAN_SANS_SC_REGULAR: &[u8] =
    include_bytes!("../assets/SourceHanSansSC-Regular.otf");
static FONT_ZHUDOU_SANS_REGULAR: &[u8] = include_bytes!("../assets/Zhudou Sans Regular.ttf");
static FONT_SOURCE_HAN_SERIF_SC_REGULAR: &[u8] =
    include_bytes!("../assets/SourceHanSerifSC-Regular.otf");
static FONT_LXGWENKAILITE_REGULAR: &[u8] = include_bytes!("../assets/LXGWWenKaiLite-Regular.ttf");
static FONT_SWEIGOTHICCJKSC_REGULAR: &[u8] =
    include_bytes!("../assets/SweiGothicCJKsc-Regular.ttf");
static FONT_TSANGER_YU_YANG_T_W02: &[u8] = include_bytes!("../assets/TsangerYuYangT-W02.ttf");
static FONT_TSANGER_YU_YANG_T_W03: &[u8] = include_bytes!("../assets/TsangerYuYangT-W03.ttf");
static FONT_FIRA_CODE_VF: &[u8] = include_bytes!("../assets/FiraCode-VF.ttf");

pub const fn get_builtin_fonts() -> [(&'static str, &'static [u8]); 8] {
    [
        (
            "Source Han Sans SC Regular",
            FONT_SOURCE_HAN_SANS_SC_REGULAR,
        ),
        ("Zhudou Sans Regular", FONT_ZHUDOU_SANS_REGULAR),
        (
            "Source Han Serif SC Regular",
            FONT_SOURCE_HAN_SERIF_SC_REGULAR,
        ),
        ("LXGWWenKaiLite Regular", FONT_LXGWENKAILITE_REGULAR),
        ("SweiGothicCJKsc Regular", FONT_SWEIGOTHICCJKSC_REGULAR),
        ("TsangerYuYangT-W02", FONT_TSANGER_YU_YANG_T_W02),
        ("TsangerYuYangT-W03", FONT_TSANGER_YU_YANG_T_W03),
        ("Fira Code VF", FONT_FIRA_CODE_VF),
    ]
}
