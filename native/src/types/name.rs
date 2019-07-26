use std::collections::HashMap;
use crate::{Context, text};
use crate::Tokenized;
use geocoder_abbreviations::TokenType;
use crate::text::titlecase;

///
/// InputName is only used internally to serialize a names array to the
/// Names type. It should not be used unless as an intermediary into or out of the Names type
///
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct InputName {
    /// Street Name
    pub display: String,

    /// When choosing which street name is primary, order by priority
    pub priority: i8
}

impl From<Name> for InputName {
    fn from(name: Name) -> Self {
        InputName {
            display: name.display,
            priority: name.priority
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Names {
    pub names: Vec<Name>
}

impl Names {
    pub fn new(names: Vec<Name>, context: &Context) -> Self {
        let mut names = Names {
            names: names
        };

        if names.names.len() == 0 {
            return names;
        }

        names.sort();

        let mut synonyms: Vec<Name> = Vec::new();

        if context.country == String::from("US") {
            for name in names.names.iter_mut() {
                synonyms.append(&mut text::syn_number_suffix(&name, &context));
                synonyms.append(&mut text::syn_written_numeric(&name, &context));
                synonyms.append(&mut text::syn_state_hwy(&name, &context));
                synonyms.append(&mut text::syn_us_hwy(&name, &context));
                synonyms.append(&mut text::syn_us_cr(&name, &context));
                synonyms.append(&mut text::syn_us_famous(&name, &context));
            }
        } else if context.country == String::from("CA") {
            for name in names.names.iter_mut() {
                synonyms.append(&mut text::syn_ca_hwy(&name, &context));

                if context.region.is_some() && context.region.as_ref().unwrap() == "QC" {
                    synonyms.append(&mut text::syn_ca_french(&name, &context));
                }
            }
        }

        names.names.append(&mut synonyms);

        names.empty();

        names
    }

    pub fn from_input(names: Vec<InputName>, context: &Context) -> Self {
        let mut full_names: Vec<Name> = Vec::with_capacity(names.len());

        for name in names {
            full_names.push(Name::new(name.display, name.priority, &context));
        }

        Names::new(full_names, &context)
    }

    ///
    /// Parse a Names object from a serde_json value, returning
    /// an empty names vec if unparseable
    ///
    pub fn from_value(value: Option<serde_json::Value>, context: &Context) -> Result<Self, String> {
        let names: Vec<Name> = match value {
            Some(street) => {
                if street.is_string() {
                    vec![Name::new(street.as_str().unwrap().to_string(), 0, &context)]
                } else {
                    let names: Vec<InputName> = match serde_json::from_value(street) {
                        Ok(street) => street,
                        Err(err) => { return Err(format!("Invalid Street Property: {}", err)); }
                    };

                    let names: Vec<Name> = names.iter().map(|name| {
                        Name::new(name.display.clone(), name.priority, &context)
                    }).collect();

                    names
                }
            },
            None => Vec::new()
        };

        Ok(Names::new(names, &context))
    }

    ///
    /// Take a second names object and add any synonyms that do not
    /// already exist on the original names object based on the
    /// tokenized version of the string.
    ///
    pub fn concat(&mut self, new_names: Names) {
        self.names.extend(new_names.names);
        self.dedupe();
    }

    ///
    /// Test to see if the given names argument has synonyms
    /// that the self names object does not
    ///
    pub fn has_diff(&self, names: &Names) -> bool {
        let mut tokenized: HashMap<String, _> = HashMap::new();

        for self_name in self.names.iter() {
            tokenized.insert(self_name.tokenized_string(), ());
        }

        for name in names.names.iter() {
            if !tokenized.contains_key(&name.tokenized_string()) {
                return true;
            }
        }

        false
    }

    ///
    /// Dedupe a names object based on the tokenized
    /// version of each name
    ///
    pub fn dedupe(&mut self) {
        let mut tokenized: HashMap<String, _> = HashMap::new();

        let mut old_names = Vec::with_capacity(self.names.len());

        loop {
            match self.names.pop() {
                Some(name) => old_names.push(name),
                None => {
                    break;
                }
            }
        }

        for name in old_names {
            if tokenized.contains_key(&name.tokenized_string()) {
                continue;
            }

            tokenized.insert(name.tokenized_string(), true);
            self.names.push(name);
        }
    }

    ///
    /// Sort names object by priority
    ///
    pub fn sort(&mut self) {
        self.names.sort_by(|a, b| {
            if a.priority > b.priority {
                std::cmp::Ordering::Less
            } else if a.priority < b.priority {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
    }

    ///
    /// Set the source on all the given names
    /// that don't have a source yet set
    ///
    pub fn set_source(&mut self, source: String) {
        for name in self.names.iter_mut() {
            if name.source == String::from("") {
                name.source = source.clone();
            }
        }
    }

    ///
    /// Remove all Name instances where display is whitespace
    ///
    pub fn empty(&mut self) {
        self.names.retain(|name| name.display.trim() != String::from(""));
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Name {
    /// Street Name
    pub display: String,

    /// When choosing which street name is primary, order by priority
    pub priority: i8,

    /// Geometry Type of a given name (network/address/generated)
    pub source: String,

    /// full token structure tokenless is derived from
    pub tokenized: Vec<Tokenized>,

    /// Frequency of the given name
    pub freq: i64
}

impl Name {
    /// Returns a representation of a street name
    ///
    /// # Arguments
    ///
    /// * `display` - A string containing the street name (Main St)
    ///
    /// ```
    pub fn new(display: impl ToString, mut priority: i8, context: &Context) -> Self {
        let mut display = display.to_string().replace(r#"""#, "");
        display = titlecase(&display, &context);

        let tokenized = context.tokens.process(&display);

        if context.country == String::from("US") || context.country == String::from("CA") {
            display = text::str_remove_octo(&display);
            // penalize less desireable street names
            if text::is_undesireable(&tokenized) {
                priority = -1;
            }
        }

        Name {
            display: display,
            priority: priority,
            source: String::from(""),
            tokenized: tokenized,
            freq: 1
        }
    }

    ///
    /// Builder style source setter
    ///
    /// ie:
    /// Name::new().set_source("generated")
    ///
    /// Can be chained with other builder functions
    ///
    pub fn set_source(mut self, source: impl ToString) -> Self {
        self.source = source.to_string();
        self
    }

    ///
    /// Builder style source setter
    ///
    /// ie:
    /// Name::new().set_freq(1)
    ///
    /// Can be chained with other builder functions
    ///
    pub fn set_freq(mut self, freq: i64) -> Self {
        self.freq = freq;
        self
    }


    pub fn tokenized_string(&self) -> String {
        let tokens: Vec<String> = self.tokenized
            .iter()
            .map(|x| x.token.to_owned())
            .collect();
        let tokenized = String::from(tokens.join(" ").trim());

        tokenized
    }

    pub fn tokenless_string(&self) -> String {
        let tokens: Vec<String> = self.tokenized
            .iter()
            .filter(|x| x.token_type.is_none())
            .map(|x| x.token.to_owned())
            .collect();
        let tokenless = String::from(tokens.join(" ").trim());

        tokenless
    }

    pub fn has_type(&self, token_type: Option<TokenType>) -> bool {
        let tokens: Vec<&Tokenized> = self.tokenized
            .iter()
            .filter(|x| x.token_type == token_type)
            .collect();

        tokens.len() > 0
    }

}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::*;
    use std::collections::HashMap;
    use crate::Tokens;

    #[test]
    fn test_name() {
        let context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        assert_eq!(Name::new(String::from("main ST nw"), 0, &context), Name {
            display: String::from("Main St NW"),
            priority: 0,
            source: String::from(""),
            tokenized: vec![
                Tokenized::new(String::from("main"), None),
                Tokenized::new(String::from("st"), None),
                Tokenized::new(String::from("nw"), None)],
            freq: 1
        });

        assert_eq!(Name::new(String::from("HiGHway #12 \" wEST"), 0, &context), Name {
            display: String::from("Highway 12 West"),
            priority: 0,
            source: String::from(""),
            tokenized: vec![
                Tokenized::new(String::from("highway"), None),
                Tokenized::new(String::from("12"), None),
                Tokenized::new(String::from("west"), None)],
            freq: 1
        });

        assert_eq!(Name::new(String::from("\thighway #12 west ext 1\n"), 0, &context), Name {
            display: String::from("Highway 12 West Ext 1"),
            priority: -1,
            source: String::from(""),
            tokenized: vec![
                Tokenized::new(String::from("highway"), None),
                Tokenized::new(String::from("12"), None),
                Tokenized::new(String::from("west"), None),
                Tokenized::new(String::from("ext"), None),
                Tokenized::new(String::from("1"), None)],
            freq: 1
        });
    }

    #[test]
    fn test_names_sort() {
        let context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        let mut names = Names::new(vec![
            Name::new(String::from("Highway 123"), -1, &context),
            Name::new(String::from("Route 123"), 2, &context),
            Name::new(String::from("Test 123"), 0, &context)
        ], &context);

        names.sort();

        let names_sorted = Names::new(vec![
            Name::new(String::from("Route 123"), 2, &context),
            Name::new(String::from("Test 123"), 0, &context),
            Name::new(String::from("Highway 123"), -1, &context)
        ], &context);

        assert_eq!(names, names_sorted);
    }

    #[test]
    fn test_names_concat() {
        let context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        let mut names = Names::new(vec![
            Name::new(String::from("Highway 123"), -1, &context),
        ], &context);

        let names2 = Names::new(vec![
            Name::new(String::from("Highway 123"), -1, &context),
            Name::new(String::from("Highway 123"), -1, &context),
        ], &context);

        names.concat(names2);

        let names_concat = Names::new(vec![
            Name::new(String::from("Highway 123"), -1, &context),
        ], &context);

        assert_eq!(names, names_concat);
    }

    #[test]
    fn test_names_dedupe() {
        let context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        let mut names = Names::new(vec![
            Name::new(String::from("Highway 123"), -1, &context),
            Name::new(String::from("Highway 123"), -1, &context),
        ], &context);

        names.dedupe();

        let names_deduped = Names::new(vec![
            Name::new(String::from("Highway 123"), -1, &context),
        ], &context);

        assert_eq!(names, names_deduped);
    }

    #[test]
    fn test_names_from_value() {
        let context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        let expected = Names::new(vec![Name::new(String::from("Main St NE"), 0, &context)], &context);

        assert_eq!(Names::from_value(Some(json!("Main St NE")), &context).unwrap(), expected);

        assert_eq!(Names::from_value(Some(json!([{
            "display": "Main St NE",
            "priority": 0
        }])), &context).unwrap(), expected);
    }

    #[test]
    fn test_names_has_diff() {
        let context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        let a_name = Names::new(vec![Name::new("Main St", 0, &context)], &context);
        let b_name = Names::new(vec![Name::new("Main St", 0, &context)], &context);
        assert_eq!(a_name.has_diff(&b_name), false);

        let a_name = Names::new(vec![Name::new("US Route 1", 0, &context)], &context);
        let b_name = Names::new(vec![Name::new("us route 1", 0, &context)], &context);
        assert_eq!(a_name.has_diff(&b_name), false);

        let a_name = Names::new(vec![Name::new("highway 1", 0, &context), Name::new("US Route 1", 0, &context)], &context);
        let b_name = Names::new(vec![Name::new("us route 1", 0, &context)], &context);
        assert_eq!(a_name.has_diff(&b_name), false);

        let a_name = Names::new(vec![Name::new("us route 1", 0, &context)], &context);
        let b_name = Names::new(vec![Name::new("highway 1", 0, &context), Name::new("US Route 1", 0, &context)], &context);
        assert_eq!(a_name.has_diff(&b_name), true);
    }

    #[test]
    fn test_names() {
        let mut context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        assert_eq!(Names::new(vec![], &context), Names {
            names: Vec::new()
        });

        assert_eq!(Names::new(vec![Name::new(String::from("Main St NW"), 0, &context)], &context), Names {
            names: vec![Name::new(String::from("Main St NW"), 0, &context)]
        });

        // Ensure invalid whitespace-only names are removed
        assert_eq!(Names::new(vec![Name::new(String::from(""), 0, &context), Name::new(String::from("\t  \n"), 0, &context)], &context), Names {
            names: Vec::new()
        });

        // Ensure synonyms are being applied correctly
        assert_eq!(Names::new(vec![Name::new(String::from("US Route 1"), 0, &context)], &context), Names {
            names: vec![
                Name::new(String::from("US Route 1"), 0, &context),
                Name::new(String::from("US Route 1"), 1, &context).set_source("generated"),
                Name::new(String::from("US 1"), -1, &context).set_source("generated"),
                Name::new(String::from("US Highway 1"), -1, &context).set_source("generated"),
                Name::new(String::from("United States Route 1"), -1, &context).set_source("generated"),
                Name::new(String::from("United States Highway 1"), -1, &context).set_source("generated"),
            ]
        });

        // Ensure highway synonyms are being applied correctly but are downgraded
        // if the highway is not the highest priority name
        assert_eq!(Names::new(vec![
            Name::new(String::from("Main St"), 0, &context),
            Name::new(String::from("US Route 1"), -1, &context)
        ], &context), Names {
            names: vec![
                Name::new(String::from("Main St"), 0, &context),
                Name::new(String::from("US Route 1"), -1, &context),
                Name::new(String::from("US Route 1"), -1, &context).set_source("generated"),
                Name::new(String::from("US 1"), -2, &context).set_source("generated"),
                Name::new(String::from("US Highway 1"), -2, &context).set_source("generated"),
                Name::new(String::from("United States Route 1"), -2, &context).set_source("generated"),
                Name::new(String::from("United States Highway 1"), -2, &context).set_source("generated"),
            ]
        });

        // @TODO remove, real world test case
        context = Context::new(String::from("us"), None, Tokens::generate(vec![String::from("en")]));

        assert_eq!(Names::new(vec![
            Name::new("NE M L King Blvd", 0, &context).set_freq(1480).set_source("address"),
            Name::new("NE MARTIN LUTHER KING JR BLVD", 0, &context).set_freq(110).set_source("address"),
            Name::new("NE M L KING BLVD", 0, &context).set_freq(18).set_source("address"),
            Name::new("SE M L King Blvd", 0, &context).set_freq(7).set_source("address"),
            Name::new("N M L King Blvd", 0, &context).set_freq(3).set_source("address"),
            Name::new("SE MARTIN LUTHER KING JR BLVD", 0, &context).set_freq(2).set_source("address"),
            Name::new("Northeast Martin Luther King Junior Boulevard", 0, &context).set_freq(1).set_source("network"),
            Name::new("NE MLK", -1, &context).set_freq(1).set_source("network"),
            Name::new("OR 99E", -1, &context).set_freq(1).set_source("network"),
            Name::new("State Highway 99E", -1, &context).set_freq(1).set_source("network")
        ], &context), Names {
            names: vec![
                Name::new("NE M L King Blvd", 0, &context).set_freq(1480).set_source("address"),
                Name::new("NE MARTIN LUTHER KING JR BLVD", 0, &context).set_freq(110).set_source("address"),
                Name::new("NE M L KING BLVD", 0, &context).set_freq(18).set_source("address"),
                Name::new("SE M L King Blvd", 0, &context).set_freq(7).set_source("address"),
                Name::new("N M L King Blvd", 0, &context).set_freq(3).set_source("address"),
                Name::new("SE MARTIN LUTHER KING JR BLVD", 0, &context).set_freq(2).set_source("address"),
                Name::new("Northeast Martin Luther King Junior Boulevard", 0, &context).set_freq(1).set_source("network"),
                Name::new("NE MLK", -1, &context).set_freq(1).set_source("network"),
                Name::new("OR 99E", -1, &context).set_freq(1).set_source("network"),
                Name::new("State Highway 99E", -1, &context).set_freq(1).set_source("network"),
                Name::new("NE Martin Luther King Jr Blvd", 1, &context).set_source("generated"),
                Name::new("NE MLK Blvd", -1, &context).set_source("generated"),
                Name::new("NE M L K Blvd", -1, &context).set_source("generated"),
                Name::new("NE Martin Luther King Blvd", -1, &context).set_source("generated"),
                Name::new("NE MLK Jr Blvd", -1, &context).set_source("generated"),
                Name::new("NE M L K Jr Blvd", -1, &context).set_source("generated"),
                Name::new("NE Martin Luther King Jr BLVD", 1, &context).set_source("generated"),
                Name::new("NE MLK BLVD", -1, &context).set_source("generated"),
                Name::new("NE M L K BLVD", -1, &context).set_source("generated"),
                Name::new("NE Martin Luther King BLVD", -1, &context).set_source("generated"),
                Name::new("NE MLK Jr BLVD", -1, &context).set_source("generated"),
                Name::new("NE M L K Jr BLVD", -1, &context).set_source("generated"),
                Name::new("NE Martin Luther King Jr BLVD", 1, &context).set_source("generated"),
                Name::new("NE MLK BLVD", -1, &context).set_source("generated"),
                Name::new("NE M L K BLVD", -1, &context).set_source("generated"),
                Name::new("NE Martin Luther King BLVD", -1, &context).set_source("generated"),
                Name::new("NE MLK Jr BLVD", -1, &context).set_source("generated"),
                Name::new("NE M L K Jr BLVD", -1, &context).set_source("generated"),
                Name::new("SE Martin Luther King Jr Blvd", 1, &context).set_source("generated"),
                Name::new("SE MLK Blvd", -1, &context).set_source("generated"),
                Name::new("SE M L K Blvd", -1, &context).set_source("generated"),
                Name::new("SE Martin Luther King Blvd", -1, &context).set_source("generated"),
                Name::new("SE MLK Jr Blvd", -1, &context).set_source("generated"),
                Name::new("SE M L K Jr Blvd", -1, &context).set_source("generated"),
                Name::new("N Martin Luther King Jr Blvd", 1, &context).set_source("generated"),
                Name::new("N MLK Blvd", -1, &context).set_source("generated"),
                Name::new("N M L K Blvd", -1, &context).set_source("generated"),
                Name::new("N Martin Luther King Blvd", -1, &context).set_source("generated"),
                Name::new("N MLK Jr Blvd", -1, &context).set_source("generated"),
                Name::new("N M L K Jr Blvd", -1, &context).set_source("generated"),
                Name::new("SE Martin Luther King Jr BLVD", 1, &context).set_source("generated"),
                Name::new("SE MLK BLVD", -1, &context).set_source("generated"),
                Name::new("SE M L K BLVD", -1, &context).set_source("generated"),
                Name::new("SE Martin Luther King BLVD", -1, &context).set_source("generated"),
                Name::new("SE MLK Jr BLVD", -1, &context).set_source("generated"),
                Name::new("SE M L K Jr BLVD", -1, &context).set_source("generated"),
                Name::new("Northeast Martin Luther King Jr Boulevard", 1, &context).set_source("generated"),
                Name::new("Northeast MLK Boulevard", -1, &context).set_source("generated"),
                Name::new("Northeast M L K Boulevard", -1, &context).set_source("generated"),
                Name::new("Northeast Martin Luther King Boulevard", -1, &context).set_source("generated"),
                Name::new("Northeast MLK Jr Boulevard", -1, &context).set_source("generated"),
                Name::new("Northeast M L K Jr Boulevard", -1, &context).set_source("generated"),
                Name::new("NE Martin Luther King Jr", -1, &context).set_source("generated"),
                Name::new("NE MLK", -2, &context).set_source("generated"),
                Name::new("NE M L K", -2, &context).set_source("generated"),
                Name::new("NE Martin Luther King", -2, &context).set_source("generated"),
                Name::new("NE MLK Jr", -2, &context).set_source("generated"),
                Name::new("NE M L K Jr", -2, &context).set_source("generated")
            ]
        });
    }

    #[test]
    fn test_tokenized_string() {
        let context = Context::new(String::from("us"), None, Tokens::generate(vec![String::from("en")]));

        assert_eq!(Name::new(String::from("Main St NW"), 0, &context).tokenized_string(),
            String::from("main st nw")
        );
        assert_eq!(Name::new(String::from("Main Street Northwest"), 0, &context).tokenized_string(),
            String::from("main st nw")
        );
    }

    #[test]
    fn test_tokenless_string() {
        let context = Context::new(String::from("us"), None, Tokens::generate(vec![String::from("en")]));

        assert_eq!(Name::new(String::from("Main St NW"), 0, &context).tokenless_string(),
            String::from("main")
        );
        assert_eq!(Name::new(String::from("Main Street Northwest"), 0, &context).tokenless_string(),
            String::from("main")
        );
        assert_eq!(Name::new(String::from("East College Road"), 0, &context).tokenless_string(),
            String::from("coll")
        );
    }

    #[test]
    fn test_has_type() {
        let context = Context::new(String::from("us"), None, Tokens::generate(vec![String::from("en")]));

        assert_eq!(Name::new(String::from("Main St NW"), 0, &context).has_type(Some(TokenType::Way)), true);
        assert_eq!(Name::new(String::from("Main St NW"), 0, &context).has_type(Some(TokenType::Cardinal)), true);
        assert_eq!(Name::new(String::from("Main St NW"), 0, &context).has_type(None), true);
        assert_eq!(Name::new(String::from("Main St NW"), 0, &context).has_type(Some(TokenType::PostalBox)), false);

        assert_eq!(Name::new(String::from("foo bar"), 0, &context).has_type(Some(TokenType::Way)), false);
        assert_eq!(Name::new(String::from("foo bar"), 0, &context).has_type(Some(TokenType::Cardinal)), false);
        assert_eq!(Name::new(String::from("foo bar"), 0, &context).has_type(None), true);
    }

    #[test]
    fn test_empty() {
        let context = Context::new(String::from("us"), None, Tokens::new(HashMap::new()));

        let mut empty_a = Names::new(vec![Name::new(String::from(""), 0, &context)], &context);
        empty_a.empty();
        assert_eq!(empty_a, Names { names: Vec::new() });

        let mut empty_b = Names::new(vec![Name::new(String::from("\t  \n"), 0, &context)], &context);
        empty_b.empty();
        assert_eq!(empty_b, Names { names: Vec::new() });

        let mut empty_c = Names::new(vec![Name::new(String::from(""), 0, &context), Name::new(String::from("\t  \n"), 0, &context)], &context);
        empty_c.empty();
        assert_eq!(empty_c, Names { names: Vec::new() });
    }
}
