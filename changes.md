Hi @sylvestre,

I am glad you liked my rework. How can we progress on this?

I already have a more advanced version with:
* added CSpell dictionaries

Implemented the coreutils logic to start the utilities:
* diffutils main like coreutils main
* added build.rs scripts
* added full uumain macro support
* added fluent internationalization
* added clap
* added lints

I am aware that commit usually are small (isolated) changes, but this is just not possible when moving the base logic.

* To make sure clap and internationalization do work, I needed one module to implement it, I used cmp.
* Changes for cmp:
  * cmp parser replaced with clap
  * cmp error messages changed to UError with internationalization
  * added locales (.ftl) for en
  * replaced extra NumberParser with coreutils Parser

Next Steps
* replace diff parser with clap parser
* move tests
* create structure for sdiff and diff3 (allowing anyone to start working on their actual functionality)

Is there anything I can do differently to make these changes happen? Any way I can support you?