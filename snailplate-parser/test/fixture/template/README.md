Fixtures must be carefully edited, since tests expect exact bytes in input, but
some editors like to add newline character at the last line, which in many cases
we do not want.

In Vim it is possible to disable newline adding with commands:
:set nofixendofline
:set noeol

Always check if file contains expected bytes with xxd command.

In this context, fixture is testing data file, that is loaded when testing
environment is set up.

Currently there are only template sub directory in fixture folder, that contains
file pairs:
1) html file - which contains input template sources as is,
2) rs file, that contains list of expected output tokens.

Both files have the same name by convention, only extension differs.


