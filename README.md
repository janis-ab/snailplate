# SnailPlate

An HTML templating engine for Rust.

## Q & A

### Is it production ready?
No it is not. It is in an early stages of development. API is unstable, expected
features are not implemented yet. If you need a templating engine right away go
find something that works.

### Why another templating engine if there so many already exist?
Yes, there are a lot of templating engines and languages out there. And there
are plenty templating engines written in Rust, but none of them do the things
in a way i would like them to be done.

I've used many different templating implementations in different programming
languages, but all of them always have lacked some features i'd like to have
in templating engine.

Just to name a few:
#### Clean and explicit code.
Generaly HTML is not very readable on it's own, and when templating engine
requires one to use more angle brackets, it becomes even less readable. In this
regard i like how variables are expanded through double curly braces in many
engines (Jinja, Laravel/blade, etc.) like so: {{ variable }}. This looks clean
when written inside HTML tags. The default <?php, <?=, etc does not look so
clean.

When one needs to use if-else, foreach loops, etc. then most of the templating
languages become ugly. In this regard i like how it is solved in Laravel/blade 
with @foreach(), @if(). This is very explicit and really looks clean. Even if
one has never seen a manual for this templating language, it is easy to 
understand from context.

I agree that what's beautiful and/or ugly is subjective point of view.

#### Good error reporting.
This is one of the most anoying things in most of templating engines. When you
forget to write some symbol or forget to close HTML tag, you get unhelpful
error message or get none at all, but the rendered HTML is broken.

This is a hard problem to be solved; that's why it is how it is. My goal is to
try my best to give helpful warnings and errors while the template is being 
parsed and compiled. I really like how Rust compiler gives warnings with helpful
suggestions.

#### Less code more done
One of the things with a lot of templating languages is that they somewhat do
not help to reduce code. For example when creating web pages it is often 
necessary to write out list of items (for example in table rows), but if there 
are no items in list, then the output is a message (for example in div) 
indicating that there are no records. There are very few templating engines that
have a special foreach clause, that is for the case when there are no items;
all other engines require you to write nested foreach inside if-else blocks.

A lot of engines do not escape variables when they are written in HTML tags; 
user has to always use a special "escape" syntax. I believe that escaping should
be applied by default and if unwanted, then user should "unescape".

### Why SnailPlate?
In a sense this is a play with words "snail and template and plate". I think it
is not necessary to explain what template means and why it is used for template
engine. Snail is used, because i expect it to be slow on compilation phase, 
since parsers that can detect errors are slower than parsers optimized for speed
but unable to recover from errors. I will always prioritize "nice error 
reporting" over speed.

Another reasoning for snail could be that, my goal is not to implement this
library right away. It is a hoby project that is done when i have a free time.

### What do you gain from sharing this code?
I don't know yet.

### How can I contribute?
At this point in time i do not expect someone to get involved, but help is 
always appreciated. If you want to get involved just contact me, we can discuss
details.

Don't write some code on your own and then send a pull request. It could be that
i am already working on the same thing, or maybe i have different idea of how
things should be implemented, etc.

I'm intending to use TDD for this project, thus various tests for implemented 
components will always be appreciated. So in this regard anyone can get 
creative, think up of many different HTML error scenarios and messages that
should be output to help user understand what the error is. I don't promise to
solve them all though, since that is not so easy task as one might think it is.

English is not my native language; sometimes i have some misspelled words in 
code or documentation. Error corrections are always appreciated.

### How often do you intend to commit something, respond to published issues?
As i am not working on this 24/7 and have other things which have higher
priority i don't intend to commit and monitor this project every day. But at the
moment i think that once a week i could update/implement something.

### What are your current goals regarding this library?
My current goal for this library is to implement basic functionality for 
templating. Since i want to allow to include files from template files, it is
necessary to start with that, because it influences overall architecture.

Then for first version i want to allow to write values through curly brace
syntax like so: {{ varname }} into buffer. That would be a good starting point.

But before all that can be done, i need to write a template parser. I am 
intending to implement each of those components in a separate module, so that 
they can be reused for other purposes as well when possible.

### What are your current goals regarding code?
My goal is to write clean and well documented code. Never to commit a functions
in main branch that has not handled all possible expected error cases.

Useful commit messages and clean change log file.