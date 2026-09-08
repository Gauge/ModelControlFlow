use mcf_core::provenance::Licence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Family {
    Permissive,
    Copyleft,
    NonCommercial,
    Bespoke,
}

impl core::fmt::Display for Family {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Permissive => "permissive",
            Self::Copyleft => "copyleft",
            Self::NonCommercial => "non-commercial",
            Self::Bespoke => "bespoke — read the terms",
        })
    }
}

const KNOWN: &[(&str, Family)] = &[
    ("mit", Family::Permissive),
    ("apache-2.0", Family::Permissive),
    ("bsd-2-clause", Family::Permissive),
    ("bsd-3-clause", Family::Permissive),
    ("isc", Family::Permissive),
    ("mpl-2.0", Family::Permissive),
    ("artistic-2.0", Family::Permissive),
    ("cc0-1.0", Family::Permissive),
    ("cc-by-4.0", Family::Permissive),
    ("gpl-2.0", Family::Copyleft),
    ("gpl-3.0", Family::Copyleft),
    ("lgpl-3.0", Family::Copyleft),
    ("agpl-3.0", Family::Copyleft),
    ("cc-by-sa-4.0", Family::Copyleft),
    ("cc-by-nc-4.0", Family::NonCommercial),
    ("cc-by-nc-sa-4.0", Family::NonCommercial),
    ("cc-by-nc-nd-4.0", Family::NonCommercial),
    ("llama2", Family::Bespoke),
    ("llama3", Family::Bespoke),
    ("llama3.1", Family::Bespoke),
    ("llama3.2", Family::Bespoke),
    ("llama3.3", Family::Bespoke),
    ("gemma", Family::Bespoke),
    ("openrail", Family::Bespoke),
    ("openrail++", Family::Bespoke),
    ("creativeml-openrail-m", Family::Bespoke),
    ("bigscience-openrail-m", Family::Bespoke),
    ("bigcode-openrail-m", Family::Bespoke),
    ("apple-ascl", Family::Bespoke),
    ("deepfloyd-if-license", Family::Bespoke),
];

const MEANS_UNMATCHED: &[&str] = &["other", "unknown", "unlicense-other", "proprietary"];

#[must_use]
pub fn recognize(declared: &str) -> Option<Licence> {
    let normalized = declared.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if MEANS_UNMATCHED.contains(&normalized.as_str()) {
        return Some(Licence::Stated);
    }
    if KNOWN
        .iter()
        .any(|(identifier, _)| *identifier == normalized)
    {
        return Some(Licence::spdx(normalized));
    }
    Some(Licence::Stated)
}

#[must_use]
pub fn family(licence: &Licence) -> Option<Family> {
    match licence {
        Licence::Spdx(identifier) => KNOWN
            .iter()
            .find(|(known, _)| *known == identifier.as_str())
            .map(|(_, family)| *family),
        _ => None,
    }
}

#[must_use]
pub fn describe(licence: Option<&Licence>) -> String {
    match licence {
        None => {
            "licence: unknown — the repository declared none, and MCF has not guessed".to_owned()
        }
        Some(Licence::Stated) => {
            "licence: terms are present and MCF could not identify them — read them before use"
                .to_owned()
        }
        Some(identified @ Licence::Spdx(name)) => match family(identified) {
            Some(family) => format!("licence: {name} ({family})"),
            None => format!("licence: {name}"),
        },
        Some(other) => format!("licence: {other}"),
    }
}

#[cfg(test)]
mod tests;
