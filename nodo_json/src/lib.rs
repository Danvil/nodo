// Copyright 2023 by David Weikersdorfer. All rights reserved.

use nodo::codelet::Codelet;
use nodo::codelet::CodeletInstance;
use nodo::codelet::Instantiate;
use nodo_core::EyreResult;
use nodo_core::WrapErr;
use std::fs::File;
use std::io::BufReader;

/// Codelets which can be instantiated with configuration loaded from a JSON file
pub trait InstantiateFromJson: Codelet + Sized {
    fn instantiate_from_json<S1: Into<String>, S2: Into<String>>(
        name: S1,
        filename: S2,
    ) -> EyreResult<CodeletInstance<Self>>;
}

impl<C> InstantiateFromJson for C
where
    C: Codelet + Default,
    <C as Codelet>::Config: for<'a> serde::Deserialize<'a>,
{
    fn instantiate_from_json<S1: Into<String>, S2: Into<String>>(
        name: S1,
        filename: S2,
    ) -> EyreResult<CodeletInstance<Self>> {
        Ok(Self::instantiate(name, load_json(filename)?))
    }
}

/// Loads an object from a JSON file
pub fn load_json<T: for<'a> serde::Deserialize<'a>, S: Into<String>>(filename: S) -> EyreResult<T> {
    let filename = filename.into();

    let reader = BufReader::new(
        File::open(&filename)
            .wrap_err_with(|| format!("error loading config file '{filename}'"))?,
    );

    let value: T = serde_json::from_reader(reader)
        .wrap_err_with(|| format!("error parsing config file '{filename}' as JSON"))?;

    Ok(value)
}
