use encoding_rs::Encoding;

pub fn get_codec_list() -> &'static [&'static Encoding] {
    use encoding_rs::*;
    static LIST: &[&Encoding; 40] = &[
        UTF_8,
        IBM866,
        ISO_8859_2,
        ISO_8859_3,
        ISO_8859_4,
        ISO_8859_5,
        ISO_8859_6,
        ISO_8859_7,
        ISO_8859_8,
        ISO_8859_8_I,
        ISO_8859_10,
        ISO_8859_13,
        ISO_8859_14,
        ISO_8859_15,
        ISO_8859_16,
        KOI8_R,
        KOI8_U,
        MACINTOSH,
        WINDOWS_874,
        WINDOWS_1250,
        WINDOWS_1251,
        WINDOWS_1252,
        WINDOWS_1253,
        WINDOWS_1254,
        WINDOWS_1255,
        WINDOWS_1256,
        WINDOWS_1257,
        WINDOWS_1258,
        X_MAC_CYRILLIC,
        GBK,
        GB18030,
        BIG5,
        EUC_JP,
        ISO_2022_JP,
        SHIFT_JIS,
        EUC_KR,
        REPLACEMENT,
        UTF_16BE,
        UTF_16LE,
        X_USER_DEFINED,
    ];

    LIST
}

pub fn decode_to_utf8(encoding: &'static Encoding, data: &[u8]) -> String {
    encoding.decode(data).0.into_owned()
}

pub fn encode_from_utf8(encoding: &'static Encoding, data: &str) -> Vec<u8> {
    encoding.encode(data).0.into_owned()
}
