//! This crate contains a new data structure that acts as a wrapper around a `HashMap`.
//! It provides its own data enum for values `(CfgValue)`, and contains multiple helper functions
//! that let you navigate the hashmap easily.
//! 
//! Its primary purpose is for configuration, allowing for validation as well. In essence, a `CfgMap`
//! would represent a configuration for an application. So far, alternatives for configuration would be 
//! to use a data format library directly, or utilise a struct that a 
//! configuration file, like JSON or TOML, would serialise into.
//! 
//! This can be more than satisfactory, especially for basic configurations, however in certain situations
//! it can prove to be more than a bit cumbersome. For example, if you plan on using default options in the case
//! that certain options aren't set, having multiple nested objects to validate and go through, etc.
//! 
//! It is very easy to make a new `CfgMap`. There are two methods:
//! 
//! ```
//! use cfgmap::CfgMap;
//! 
//! let map1 = CfgMap::new();
//! let map2 = CfgMap::with_default("default".into());
//! ```
//! 
//! `CfgMap` allows for some functionality with regards to default values. With the `new()` function, the location 
//! for default values is in the root, whereas with `with_default` it is wherever you set it. 
//! 
//! `CfgMap` also comes with support for a certain `path` syntax with its keys:
//! 
//! ```
//! # use cfgmap::CfgMap;
//! # let cfgmap = CfgMap::new();
//! cfgmap.get("hello/there/pal");
//! ```
//! 
//! This helps to make access to nested items easy. The line above is essentially equal to:
//! 
//! ```
//! # use cfgmap::CfgMap;
//! # let map = CfgMap::new();
//! map.get("hello")
//!     .and_then(|a| a.as_map())
//!     .and_then(|a| a.get("there"))
//!     .and_then(|a| a.as_map())
//!     .and_then(|a| a.get("pal"));
//! ```
//! 
//! Note that if `hello` or `there` weren't `CfgMap`s as well, the whole expression would evaluate to `None`.
//! 
//! Now, what if you want to check what a certain value evaluates to? This is something that you'll encounter 
//! very quickly if you'd like to use any value. This crate comes with an extensive support for `Conditions`!
//! 
//! ```
//! # use cfgmap::CfgMap;
//! use cfgmap::{Condition::*, Checkable};
//! # let cfgmap = CfgMap::new();
//! let is_number = cfgmap.get("hello/there/pal").check_that(IsInt | IsFloat);
//! ```
//! 
//! The above line will check whether the value at `hello/there/pal` is a `CfgValue::Int` or a `CfgValue::Float`.
//! There are more conditions listed [*here*](./enum.Condition.html). If there are more conditions that you'd like added,
//! feel free to open up an issue or open a PR! All of these serve as utilities to help validate a certain value.
//! 
//! Defaults can also be used quite easily:+
//! 
//! ```
//! # use cfgmap::CfgMap;
//! # let map = CfgMap::new();
//! map.get_option("http_settings", "ip_address");
//! ```
//! 
//! Let's say that `map` was initialised with its default at `default`. The above line will be equivalent to the following:
//! 
//! ```
//! # use cfgmap::CfgMap;
//! # let map = CfgMap::new();
//! map.get("http_settings/ip_address").or(map.get("default/ip_address"));
//! ```
//! 
//! You can also update an option like this, using `update_option`. This works similar to using `add`, except that it doesn't 
//! add a new option if it isn't found, only updating an existing one.

use std::collections::HashMap;
mod conditions;
pub use conditions::{Checkable, Condition};
use std::concat;
use std::mem;
use std::ops::Deref;
use std::ops::DerefMut;

// The type contained within `CfgValue::Int`
pub(crate) type _Int = isize;

// The type contained within `CfgValue::Float`
pub(crate) type _Float = f64;

// The type contained within `CfgValue::Str`
pub(crate) type _Str = String;

/// The type contained within `CfgValue::Bool`
pub(crate) type _Bool = bool;

macro_rules! doc_comment {
    ($x:expr, $($tt:tt)*) => {
        #[doc = $x]
        $($tt)*
    };
}

macro_rules! is_type {
    ($fn_name:ident, $enum_type:path) => {
        doc_comment! {
            concat!("Checks whether the enum is a `", stringify!($enum_type), "`."),
            pub fn $fn_name (&self) -> bool {
                if let $enum_type(_) = self {
                    true
                } else { false }
            }
        }
    };
}

macro_rules! as_type {
    ($fn_name:ident, $type:ty, $enum_type:path) => {
        doc_comment! {
            concat!("Returns a reference to the `", stringify!($type),
                    "`. Result is `None` if contents aren't a `", stringify!($enum_type), "`."),
            pub fn $fn_name (&self) -> Option<&$type> {
                if let $enum_type(x) = self {
                    Some(x)
                } else { None }
            }
        }
    };
}

macro_rules! as_mut_type {
    ($fn_name:ident, $type:ty, $enum_type:path) => {
        doc_comment! {
            concat!("Returns a reference to the `", stringify!($type),
                    "`. Result is `None` if contents aren't a `", stringify!($enum_type), "`."),
            pub fn $fn_name (&mut self) -> Option<&mut $type> {
                if let $enum_type(x) = self {
                    Some(x)
                } else { None }
            }
        }
    };
}

/// Represents a value within a `CfgMap`
/// 
/// **EXTRA STUFF HERE**
#[derive(Debug, Clone, PartialEq)]
pub enum CfgValue {
    /// Represents an integer value.
    Int(_Int),

    /// Represents a float value.
    Float(_Float),

    /// Represents a string.
    Str(_Str),

    /// Represents a bool.
    Bool(_Bool),

    /// Represents a nested configuration map.
    Map(CfgMap),

    /// Represents a list of values. These values can have differing types.
    List(Vec<CfgValue>),
}

impl CfgValue {
    /// Returns the contents of the enum converted into an integer, if possible.
    /// 
    /// If the enum represents a float, it will be converted into an integer.
    pub fn to_int(&self) -> Option<_Int> {
        if let CfgValue::Int(x) = self {
            Some(*x)
        } else if let CfgValue::Float(x) = self {
            Some(*x as _Int)
        } else { None }
    }

    /// Returns the contents of the enum converted into a float, if possible.
    /// 
    /// If the enum represents an integer, it will be converted into a float.
    pub fn to_float(&self) -> Option<_Float> {
        if let CfgValue::Float(x) = self {
            Some(*x)
        } else if let CfgValue::Int(x) = self {
            Some(*x as _Float)
        } else { None }
    }

    is_type!(is_int, CfgValue::Int);
    is_type!(is_float, CfgValue::Float);
    is_type!(is_str, CfgValue::Str);
    is_type!(is_map, CfgValue::Map);
    is_type!(is_list, CfgValue::List);

    as_type!(as_int, _Int, CfgValue::Int);
    as_type!(as_float, _Float, CfgValue::Float);
    as_type!(as_str, _Str, CfgValue::Str);
    as_type!(as_bool, _Bool, CfgValue::Bool);
    as_type!(as_map, CfgMap, CfgValue::Map);
    as_type!(as_list, Vec<CfgValue>, CfgValue::List);

    as_mut_type!(as_int_mut, _Int, CfgValue::Int);
    as_mut_type!(as_float_mut, _Float, CfgValue::Float);
    as_mut_type!(as_str_mut, _Str, CfgValue::Str);
    as_mut_type!(as_bool_mut, _Bool, CfgValue::Bool);
    as_mut_type!(as_map_mut, CfgMap, CfgValue::Map);
    as_mut_type!(as_list_mut, Vec<CfgValue>, CfgValue::List);
}

impl conditions::Checkable for CfgValue {
    fn check_that(&self, c: conditions::Condition) -> bool {
        return c.execute(self).to_bool();
    }
}

impl conditions::Checkable for Option<&CfgValue> {
    fn check_that(&self, condition: conditions::Condition) -> bool {
        self.as_ref().map_or(false, |val| val.check_that(condition))
    }
}

impl conditions::Checkable for Option<&mut CfgValue> {
    fn check_that(&self, condition: conditions::Condition) -> bool {
        self.as_ref().map_or(false, |val| val.check_that(condition))
    }
}

fn split_once(in_string: &str, pat: char) -> (String, Option<String>) {
    if in_string.find(pat).is_none() {
        return (in_string.into(), None);
    }

    let mut splitter = in_string.splitn(2, pat);
    let first = splitter.next().unwrap().to_string();
    let second = splitter.next().unwrap().to_string();

    (first, Some(second))
}

fn rsplit_once(in_string: &str, pat: char) -> (Option<String>, String) {
    if in_string.find(pat).is_none() {
        return (None, in_string.into());
    }

    let mut splitter = in_string.rsplitn(2, pat);
    let first = splitter.next().unwrap().to_string();
    let second = splitter.next().unwrap().to_string();

    (Some(second), first)
}

impl Deref for CfgMap {
    type Target = HashMap<String, CfgValue>;

    fn deref(&self) -> &Self::Target {
        &self.internal_map
    }
}

impl DerefMut for CfgMap {
    fn deref_mut (&mut self) -> &mut Self::Target {
        &mut self.internal_map
    }
}


/// A configuration map, containing helper functions and effectively being a wrapper
/// around a `HashMap`s.
/// 
/// **TODO: FILL THIS IN**
#[derive(Debug, Clone, PartialEq)]
pub struct CfgMap {
    /// An internal map representing the configuration.
    internal_map: HashMap<String, CfgValue>,

    /// A path to the default subobject.
    default: String
}

impl CfgMap {

    /// Creates a new empty CfgMap.
    pub fn new() -> CfgMap {
        CfgMap { internal_map: HashMap::new(), default: String::new() }
    }

    /// Creates a new empty CfgMap with a default directory.
    pub fn with_default(path: String) -> CfgMap {
        CfgMap { internal_map: HashMap::new(), default: format!("{}/", path) }
    }

    /// Adds a new entry in the configuration.
    /// 
    /// The `key` can be of the form of the path `"a/b/...y/z/"`, in which case it will
    /// get the inner submap `a/b/...y/`, and add `z` onto it. This is for convenience sake,
    /// as doing this manually can prove to be verbose.
    /// 
    /// In order to add a default value to a normal submap - you would need to do this manually,
    /// as this function will always use `get_mut`.
    /// 
    /// ## Examples
    /// 
    /// ```
    /// use cfgmap::{CfgMap, CfgValue::*};
    /// 
    /// let mut cmap = CfgMap::new();
    /// 
    /// // Works - a root add like this will always work.
    /// assert!(cmap.add("k1", Int(5)).is_ok());
    /// 
    /// // Doesn't work, because k1 isn't a map.
    /// assert!(cmap.add("k1/k2", Int(10)).is_err());
    /// 
    /// // Works - returns the old value.
    /// let r = cmap.add("k1", Float(8.0));
    /// assert_eq!(Ok(Some(Int(5))), r);
    /// ```
    /// 
    /// ## Return values
    /// 
    /// - `Err` if the path as specified by `key` isn't found. In the case above for example, `get_mut("a")` returns a `None`.
    /// - `Ok(Some(CfgValue))` if the path as specified by key already contained a value, and was overwritten. In this case, the old value is returned.
    /// - `Ok(None)` otherwise.
    pub fn add(&mut self, key: &str, value: CfgValue) -> Result<Option<CfgValue>, ()> {
        let (path, key) = rsplit_once(key, '/');

        if path.is_none(){
            Ok(self.internal_map.insert(key.to_string(), value))
        }
        else {
            let subtree = self.get_mut(&path.unwrap());
            if subtree.check_that(Condition::IsMap) {
                subtree.unwrap().as_map_mut().unwrap().add(&key, value)
            }
            else {
                Err(())
            }
        }
    }

    /// Gets a reference to a value from within the configuration.
    /// 
    /// The `key` can be of the form of the path `"a/b/...y/z/"`, in which case it will
    /// go through the inner submaps `"a/b/..."` until a submap isn't found, or the end is reached.
    /// This is for convenience sake, as doing this manually can prove to be verbose.
    /// 
    /// Returns `None` if the key doesn't exist.
    /// 
    /// ## Examples
    /// ```
    /// use cfgmap::{CfgMap, CfgValue::*, Condition::*, Checkable};
    /// 
    /// let mut cmap = CfgMap::new();
    /// let mut submap = CfgMap::new();
    /// 
    /// submap.add("key", Int(5));
    ///
    /// cmap.add("sub", Map(submap));
    /// 
    /// assert!(cmap.get("sub").check_that(IsMap));
    /// assert!(cmap.get("sub/key").check_that(IsExactlyInt(5)));
    /// ```
    pub fn get(&self, key: &str) -> Option<&CfgValue> {
        let (h, t) = split_once(key, '/');

        if t.is_none() {
            self.internal_map.get(key)
        }
        else {
            self.internal_map.get(&h).and_then(|op| {
                op.as_map()
            }).and_then(|map| {
                map.get(&t.unwrap())
            })
        }
    }

    /// Gets a mutable reference to a value from within the configuration.
    /// 
    /// Returns `None` if the key doesn't exist.
    /// 
    /// The `key` can be of the form of the path `"a/b/...y/z/"`, in which case it will
    /// go through the inner submaps `"a/b/..."` until a submap isn't found, or the end is reached.
    /// This is for convenience sake, as doing this manually can prove to be verbose.
    /// 
    /// ## Examples
    /// ```
    /// use cfgmap::{CfgMap, CfgValue::*, Condition::*, Checkable};
    /// 
    /// let mut cmap = CfgMap::new();
    /// let mut submap = CfgMap::new();
    ///
    /// cmap.add("sub", Map(submap));
    /// 
    /// let mut submap = cmap.get_mut("sub");
    /// assert!(submap.check_that(IsMap));
    /// 
    /// submap.unwrap().as_map_mut().unwrap().add("key", Int(5));
    /// assert!(cmap.get_mut("sub/key").check_that(IsExactlyInt(5)));
    /// ```
    pub fn get_mut(&mut self, key: &str) -> Option<&mut CfgValue> {
        let (h, t) = split_once(key, '/');

        if t.is_none() {
            self.internal_map.get_mut(key)
        }
        else {
            self.internal_map.get_mut(&h).and_then(|op| {
                op.as_map_mut()
            }).and_then(|map| {
                map.get_mut(&t.unwrap())
            })
        }
    }

    /// Checks whether a certain path exists.
    /// 
    /// The `key` can be of the form of the path `"a/b/...y/z/"`, in which case it will
    /// go through the inner submaps `"a/b/..."` until a submap isn't found, or the end is reached.
    /// This is for convenience sake, as doing this manually can prove to be verbose.
    /// 
    /// ## Examples
    /// ```
    /// use cfgmap::{CfgMap, CfgValue::*, Condition::*, Checkable};
    /// 
    /// let mut cmap = CfgMap::new();
    /// let mut submap = CfgMap::new();
    ///
    /// cmap.add("num", Int(10));
    /// submap.add("num", Int(20));
    /// cmap.add("sub", Map(submap));
    /// 
    /// assert!(cmap.contains_key("num"));
    /// assert!(cmap.contains_key("sub/num"));
    /// ```
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Gets a reference to an option within the configuration.
    /// 
    /// It first tries to get 
    /// `category/option` within the normal values. If this doesn't exist, it will then 
    /// try to retrieve `option` from the default path instead (`self.default/option`).
    /// 
    /// Note that if `default` wasn't set on construction, this function will instead retrieve
    /// the value from the root directory (`option`) directly.
    /// 
    /// Returns `None` if the key doesn't exist in either map.
    /// 
    /// The `key` can be of the form of the path `"a/b/...y/z/"`, in which case it will
    /// go through the inner submaps `"a/b/..."` until a submap isn't found, or the end is reached.
    /// This is for convenience sake, as doing this manually can prove to be verbose.
    /// 
    /// ## Examples
    /// ```
    /// use cfgmap::{CfgMap, CfgValue::*, Checkable, Condition::*};
    /// 
    /// let mut cmap = CfgMap::new();
    /// let mut submap = CfgMap::new();
    /// 
    /// submap.add("OP1", Int(5));
    /// cmap.add("OP1", Int(8));
    /// 
    /// cmap.add("sub", Map(submap));
    /// 
    /// assert!(cmap.get_option("sub", "OP1").check_that(IsExactlyInt(5)));
    /// assert!(cmap.get_option("sub", "OP1").check_that(IsExactlyInt(5)));
    /// assert!(cmap.get_option("sub", "OP2").is_none());
    /// ```
    pub fn get_option(&self, category: &str, option: &str) -> Option<&CfgValue> {
        let fullkey = format!("{}/{}", category, option);
        let default = format!("{}{}", self.default, option);
        self.get(&fullkey).or(self.get(&default))
    }

    /// Updates the option with the new value `to`.
    /// 
    /// It first tries to get 
    /// `category/option` within the normal values. If this doesn't exist, it will then 
    /// try to retrieve `option` from the default path instead (`self.default/option`).
    /// 
    /// Note that if `default` wasn't set on construction, this function will instead retrieve
    /// the value from the root directory (`option`) directly.
    /// 
    /// The `key` can be of the form of the path `"a/b/...y/z/"`, in which case it will
    /// go through the inner submaps `"a/b/..."` until a submap isn't found, or the end is reached.
    /// This is for convenience sake, as doing this manually can prove to be verbose.
    /// 
    /// ## Examples
    /// ```
    /// use cfgmap::{CfgMap, CfgValue::*, Checkable, Condition::*};
    /// 
    /// let mut cmap = CfgMap::new();
    /// let mut submap = CfgMap::new();
    /// 
    /// submap.add("OP1", Int(5));
    /// cmap.add("OP1", Int(8));
    /// 
    /// cmap.add("sub", Map(submap));
    /// 
    /// let OL1 = cmap.update_option("sub", "OP1", Int(10));
    /// let OL2 = cmap.update_option("foo", "OP1", Int(16));
    /// let OL3 = cmap.update_option("sub", "OP2", Int(99));
    /// 
    /// assert!(cmap.get_option("sub", "OP1").check_that(IsExactlyInt(10)));
    /// assert!(cmap.get_option("foo", "OP1").check_that(IsExactlyInt(16)));
    /// assert!(cmap.get_option("sub", "OP2").is_none());
    /// 
    /// assert_eq!(OL1, Some(Int(5)));
    /// assert_eq!(OL2, Some(Int(8)));
    /// assert_eq!(OL3, None);
    /// ```
    pub fn update_option(&mut self, category: &str, option: &str, to: CfgValue) -> Option<CfgValue> {
        let fullkey = format!("{}/{}", category, option);
        let default = format!("{}{}", self.default, option);

        if let Some(x) = self.get_mut(&fullkey) {
            Some(mem::replace(x, to))
        } else if let Some(x) = self.get_mut(&default) {
            Some(mem::replace(x, to))
        } else {
            None
        }
    }
}