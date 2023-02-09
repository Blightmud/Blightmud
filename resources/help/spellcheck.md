# Spellcheck

This module is not a complete spellchecking solution. It provides 
methods for scripts to check whether a word is misspelled, or to
get suggestions for a misspelled word. Before use, the spellcheck 
module must be initialized by providing paths to a Hunspell 
compatible AFF file, and dictionary file (not included).

Complete spellchecking functionality is available by installing 
a plugin that uses these APIs, for example:

- [Blightspell](https://github.com/cpu/blightspell)

##

***spellcheck.init(aff_path, dict_path)***
Initializes spellchecking using the provided paths.

- `aff_path`    path to a Hunspell affix file.
- `dict_path`   path to a Hunspell dictionary file.

##

***spellcheck.check(word) -> bool***
Checks whether the given word exists in the dictionary. If called
before `spellcheck.init` an error will be produced.

- `word`    A potentially misspelled word.
- Returns true if the spelling is correct, otherwise false.

##

***spellcheck.suggest(word) -> table***
Returns a table of suggested replacements for a misspelled word. 
If called before `spellcheck.init` an error will be produced.

- `word`    A potentially misspelled word.
- Returns a table array consisting of candidate replacement words, in 
  order of likelihood.

