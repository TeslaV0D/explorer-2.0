//! Access keys per language (core::mnemonic): in every catalog the mnemonic
//! text without `&` equals the label the dialog shows, and the keys of one
//! dialog differ. Out of step? Fix `lang/<code>/…/explorer2.po`.

use explorer_core::mnemonic::{key_of, strip, ConflictKey, CONFLICT};
use explorer_i18n::translate_in;

#[test]
fn conflict_keys_match_labels_and_differ() {
    let mut codes = explorer_i18n::compiled_languages();
    codes.push("en");
    for code in codes {
        for (m, label, _) in CONFLICT {
            let t = translate_in(code, m);
            assert_eq!(
                strip(t),
                translate_in(code, label),
                "{code}: {m:?} = {t:?} must be the label {label:?} with a '&'"
            );
            assert!(key_of(t).is_some(), "{code}: {t:?} has no '&' key");
        }
        // the dialog shows either Replace or Merge, never both
        for slot0 in ["Replace", "Merge"] {
            let keys: Vec<char> = CONFLICT
                .iter()
                .filter(|(_, l, k)| *k != ConflictKey::Replace || *l == slot0)
                .map(|(m, _, _)| key_of(translate_in(code, m)).unwrap())
                .collect();
            let mut unique = keys.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), keys.len(), "{code}: keys {keys:?} collide");
        }
    }
}
