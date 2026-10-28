"""Load classification of one visible Bash command for the window hook (#1049).

tools/fenster-hook.py imports this module only while another session holds the
measurement window (or the switch is unreadable), so ordinary Bash calls never
pay for compiling it. Nothing here executes the command.

analyse(command, cwd) returns (kinds, error): which shared resource the command
visibly loads — LOKAL, this machine; BAU, the build service, which runs on node3
next to the measurements there (rota#1049, 28.10.) — and the first reason a
part of it could not be decided: unsplittable shell text (Bash itself would
reject it), code piped into an interpreter, or a program whose name is only
known at run time ("$B" is mostly a Rust test binary here). load_command()
keeps the old answer: True for any load, ValueError if nothing visible loads
and something was undecidable.

The lexer follows Bash quoting, $( ) and backquote substitutions (also inside
double quotes), process substitution, comments, case patterns and heredocs far
enough to separate commands from data. A heredoc body is data unless the shell
reads it as code (bash/sh/zsh), unless Python/Node code in it visibly starts a
tools/ file, a load tool or a Rust binary, or unless an unquoted delimiter lets
the shell expand $( ) and backquotes in it. It is not a shell: aliases and
shell functions defined elsewhere remain out of scope.

A script that is run (`bash x.sh`, `python3 x.py`, `source x.sh`, `./x.sh`) is
judged by what it runs: by the heredoc that wrote it earlier in the same
command, else by the file itself, read relative to the session's working
directory. A script that is neither is undecidable — the review of 24.10.
found `cat > x.sh <<EOF … EOF; bash x.sh` quiet in a foreign window.
"""

import os
import re

LOAD_TOOLS = {'npm', 'npx', 'pnpm', 'yarn', 'cargo', 'cargo-remote-fmt',
              'rustc', 'vitest', 'prettier', 'rota-sim', 'tsc', 'eslint', 'vite', 'tsx'}
# Help/version flags per load tool. Cargo Remote v2 uses -h for --sync-hidden,
# so `cargo remote -h -- build` still runs remotely. `cargo-remote-fmt -h` is
# help: its format mode parses with argparse's own -h before any remote action
# (measured 28.10.: usage, exit 0, no job).
HELP = {'npm': {'--help', '-h', '--version', '-v'},
        'pnpm': {'--help', '-h', '--version', '-v'},
        'yarn': {'--help', '-h', '--version', '-v'},
        'npx': {'--help', '-h', '--version'},
        'cargo': {'--help', '-h', '--version', '-V', '--list'},
        'cargo-remote-fmt': {'--help', '-h', '--version'},
        'rustc': {'--help', '-h', '--version', '-V'},
        'vitest': {'--help', '-h', '--version', '-v'},
        'prettier': {'--help', '-h', '--version', '-v'},
        'rota-sim': {'--help', '-h', '--version', '-V'},
        'tsc': {'--help', '-h', '--version', '-v'},
        'eslint': {'--help', '-h', '--version', '-v'},
        'vite': {'--help', '-h', '--version', '-v'},
        'tsx': {'--help', '-h', '--version', '-v'}}
# Wrapper: (short options taking a value, long options taking a value,
#           positional words before the wrapped command).
WRAPPERS = {'env': ('uCS', {'--unset', '--chdir', '--split-string'}, 0),
            'sudo': ('CDghpRrTtUu', {'--chdir', '--group', '--host', '--prompt', '--user',
                                     '--close-from', '--chroot', '--role', '--type',
                                     '--other-user', '--command-timeout'}, 0),
            'command': ('', set(), 0), 'builtin': ('', set(), 0),
            'exec': ('a', set(), 0), 'time': ('fo', {'--format', '--output'}, 0),
            'timeout': ('ks', {'--kill-after', '--signal'}, 1),
            'nice': ('n', {'--adjustment'}, 0), 'nohup': ('', set(), 0),
            'setsid': ('', set(), 0), 'stdbuf': ('ioe', {'--input', '--output', '--error'}, 0),
            'ionice': ('cnpPu', {'--class', '--classdata', '--pid', '--pgid', '--uid'}, 0),
            'taskset': ('', set(), 1),
            'xargs': ('adEILnPs', {'--arg-file', '--delimiter', '--max-args', '--max-procs',
                                   '--max-chars', '--process-slot-var'}, 0),
            # `flock [-w s] FILE CMD …` or `FILE -c 'CMD'`; `flock FD` starts none.
            # Our own form for heavy runs, and quiet until the review of 24.10.
            'flock': ('wE', {'--wait', '--timeout', '--conflict-exit-code'}, 1)}
# Options after which a wrapper starts no command (query, existing process).
NO_COMMAND = {'command': 'vV', 'ionice': 'pPu', 'taskset': 'p', 'sudo': 'eKlvV',
              'flock': 'hV'}
NO_COMMAND_LONG = {'--help', '--version', '--pid', '--pgid', '--uid', '--list',
                   '--validate', '--edit', '--remove-timestamp'}
RESERVED = {'!', '{', '}', 'if', 'then', 'elif', 'else', 'fi', 'while', 'until',
            'do', 'done', 'esac'}
SHELLS = {'bash', 'sh', 'zsh'}
CODE_RUNNERS = {'python3', 'python', 'node'}
MAX_DEPTH = 8          # bash -c, eval and interpreter heredocs inside each other
MAX_NESTING = 16       # $( ), <( ) and backquotes inside each other
DYNAMIC = '\x00'       # stands for text that is only known at run time

ASSIGNMENT = re.compile(r'[A-Za-z_][A-Za-z0-9_]*(?:\[[^]]*\])?\+?=')
TOOLS_PATH = re.compile(r'(?:^|[^A-Za-z0-9_.-])tools/')
TARGET_BIN = re.compile(r'(?:^|/)target/(?:debug|release)/(?:examples/|deps/)')
RECORDINGS = re.compile(r'(?:^|[\s/])recordings/')
SUBCOMMAND = re.compile(r'[a-z][a-z0-9-]*')
OPERATOR = re.compile(r';;&|;;|;&|&&|\|\||\|&|[;&|]')
REDIRECT = re.compile(r'[0-9]*(?:<<<|<<-|<<|&>>|&>|>>|>&|<&|<>|>\||[<>])')
PLAIN = re.compile(r'[^\s;&|()<>\\\'"$`]+')
PLAIN_DOUBLE = re.compile(r'[^"\\$`]+')
BLANKS = re.compile(r'(?:[ \t\r]|\\\n)+')
PARAMETER = re.compile(r'[A-Za-z_][A-Za-z0-9_]*|[0-9@*#?$!-]')
ANSI_C = re.compile(r"(?:[^'\\]|\\.)*'", re.DOTALL)
SPECIAL = re.compile(r'[\\$`]')
BACKQUOTE_END = re.compile(r'[`\\]')
QUOTING = re.compile(r'[\'"\\]')
SHELL_CODE = re.compile(r'-[A-Za-z]*c[A-Za-z]*')
SHELL_STDIN = re.compile(r'-[A-Za-z]*s[A-Za-z]*')
SHELL_VALUED = {'-o', '+o', '-O', '+O', '--rcfile', '--init-file'}
CODE_VALUED = {'-W', '-X', '-r', '--require', '--import', '--loader'}
INLINE_CODE = re.compile(r'-[A-Za-z]*[ce]|--eval|-p|--print')
# Inline Python/Node code is data unless it visibly starts something (a process,
# an import or exec of a file) and what it starts is a tools/ file, a load tool
# named in a string literal or a locally built Rust binary. Reading or patching
# a file below tools/ is not a tool run.
STARTS = re.compile(r'\b(?:subprocess|os\.system|os\.popen|Popen|check_output|check_call|runpy'
                    r'|run_path|import_module|spec_from_file_location|exec_module|exec|execSync'
                    r'|execFileSync|spawnSync|spawn|execFile)\b')
CODE_TARGET = re.compile(r'(?:^|[^A-Za-z0-9_.-])target/(?:debug|release)/(?:examples|deps)/')
# A load tool reached by its installed path: node_modules/.bin/<tool> or the
# package directory itself (`node node_modules/vitest/vitest.mjs`).
NODE_TOOL = re.compile(r'(?:^|/)node_modules/(?:\.bin/(?:tsc|eslint|vite|vitest|prettier|tsx)(?:$|[^A-Za-z0-9_-])'
                       r'|(?:typescript|eslint|vite|vitest|prettier|tsx)/)')
LOKAL, BAU = 'lokal', 'bau-dienst'
# `cargo remote` and `cargo-remote-fmt` load the build service, not this
# machine; the queries among them start no job at all.
REMOTE_QUERY = {'--status', '--logs', '--cancel', '--resume'}
REMOTE_LITERAL = re.compile(r"""cargo-remote|cargo\s+remote\b|cargo['"]\s*,\s*['"]remote\b""")
LOCAL_LITERAL = re.compile(r"""['"`](?:npm|npx|pnpm|yarn|rustc|vitest|prettier|rota-sim|tsc|eslint|vite|tsx)\b"""
                           r"""|['"`]cargo\b(?!-remote)(?!\s+remote\b)(?!['"]\s*,\s*['"]remote\b)""")
TOOLS_FILE = re.compile(r'(?:^|[^A-Za-z0-9_.-])(tools/[A-Za-z0-9_./-]+)')
OUTPUTS = {'>', '>>', '>|', '&>', '&>>'}
SCRIPT_LIMIT = 256 * 1024     # a larger script is not read; it is undecidable


class ShellError(ValueError):
    """The visible text does not decide whether the command loads the machine."""


class Context:
    """What every rule of one hook call shares: the text being judged, the
    session's working directory, the files this command writes from a heredoc
    and the first undecidable part. A script read along the way gets a child
    context with its own text and everything else shared."""

    __slots__ = ('text', 'cwd', 'written', 'errors')

    def __init__(self, text, cwd=None, written=None, errors=None):
        self.text, self.cwd = text, cwd
        self.written = {} if written is None else written
        self.errors = [] if errors is None else errors

    def child(self, text):
        return Context(text, self.cwd, self.written, self.errors)

    def note(self, error):
        self.errors.append(error)

    def path(self, name):
        name = os.path.expanduser(name)
        return os.path.normpath(name if os.path.isabs(name) or not self.cwd
                                else os.path.join(self.cwd, name))


class Segment:
    """One simple command: its words plus the data its redirections supply."""

    __slots__ = ('words', 'heredocs', 'strings', 'inputs', 'outputs', 'piped')

    def __init__(self, piped=False):
        self.words, self.heredocs, self.strings, self.inputs, self.outputs = [], [], [], [], []
        self.piped = piped


class Lexer:
    """Split shell text into simple commands; heredoc bodies stay data."""

    def __init__(self, text, found, nesting=0):
        self.text, self.pos, self.found, self.nesting = text, 0, found, nesting
        self.pending = []   # heredocs announced before the next newline

    def commands(self, stop=None):
        """Lex until `stop` (the ')' closing a substitution) or the end of the text."""
        self.nesting += 1
        if self.nesting > MAX_NESTING:
            raise ShellError('substitution nesting exceeds classifier limit')
        text, size = self.text, len(self.text)
        segment, parens, cases = Segment(), 0, []
        while True:
            self.blanks()
            if self.pos >= size:
                if stop or parens or cases:
                    raise ShellError('unexpected end of shell text')
                self.finish(segment)
                self.heredocs()
                break
            char = text[self.pos]
            if cases and cases[-1] == 'pattern':
                # Case patterns are data: ')' starts the branch, 'esac' ends the case.
                if char in '\n(|':
                    self.pos += 1
                    if char == '\n':
                        self.heredocs()
                elif char == ')':
                    self.pos += 1
                    cases[-1] = 'body'
                elif self.word() == 'esac':
                    cases.pop()
                continue
            if char == '\n':
                self.pos += 1
                segment = self.finish(segment)
                self.heredocs()
            elif char == ')':
                self.pos += 1
                segment = self.finish(segment)
                if parens:
                    parens -= 1
                elif stop:
                    break
                else:
                    raise ShellError('unbalanced parenthesis')
            elif char == '(':
                self.pos += 1
                if text.startswith('(', self.pos) and all(
                        word in RESERVED or word == 'for' for word in segment.words):
                    self.pos += 1
                    self.arithmetic()      # (( ... )) and for (( ... )) start no command
                else:
                    segment = self.finish(segment)
                    parens += 1
            elif char in '<>' and text.startswith('(', self.pos + 1):
                self.pos += 2
                segment.words.append(self.substitution())   # bash <(git show ...:tools/x.sh)
            elif match := REDIRECT.match(text, self.pos):
                self.pos = match.end()
                self.redirect(match.group().lstrip('0123456789'), segment)
            elif match := OPERATOR.match(text, self.pos):
                self.pos = match.end()
                segment = self.finish(segment, match.group() in ('|', '|&'))
                if match.group() in (';;', ';&', ';;&') and cases:
                    cases[-1] = 'pattern'
            else:
                word = self.word()
                if word == 'esac' and cases and all(item in RESERVED for item in segment.words):
                    cases.pop()
                    continue
                segment.words.append(word)
                if (word == 'in' and len(segment.words) >= 3 and segment.words[-3] == 'case'
                        and all(item in RESERVED for item in segment.words[:-3])):
                    cases.append('pattern')
                    segment = Segment()
        self.nesting -= 1

    def finish(self, segment, piped=False):
        if segment.words:
            self.found.append(segment)
        elif not (segment.heredocs or segment.strings or segment.inputs or segment.outputs
                  or any(item[3] is segment for item in self.pending)):
            segment.piped = segment.piped or piped   # e.g. a newline after '|'
            return segment
        return Segment(piped)

    def blanks(self):
        """Skip blanks, line continuations and comments; a comment ends at '\\n'."""
        text = self.text
        while True:
            match = BLANKS.match(text, self.pos)
            if match:
                self.pos = match.end()
            if not text.startswith('#', self.pos):
                return
            end = text.find('\n', self.pos)
            self.pos = len(text) if end < 0 else end

    def redirect(self, operator, segment):
        self.blanks()
        if operator in ('<<', '<<-'):
            start = self.pos
            self.word()
            raw = self.text[start:self.pos]
            delimiter = QUOTING.sub('', raw)
            self.pending.append((delimiter, operator == '<<-', delimiter != raw, segment))
            return
        if self.text.startswith(('<(', '>('), self.pos):
            self.pos += 2
            value = self.substitution()
        else:
            value = self.word()
        if operator == '<<<':
            segment.strings.append(value)
        elif operator in ('<', '<>'):
            segment.inputs.append(value)
        elif operator in OUTPUTS:
            segment.outputs.append(value)

    def substitution(self):
        """Lex the commands of $( ), <( ) or >( ); the word keeps their text."""
        start = self.pos
        self.commands(')')
        return DYNAMIC + self.text[start:self.pos - 1]

    def heredocs(self):
        """Read the bodies announced on the line that just ended; they are data."""
        text, size = self.text, len(self.text)
        for delimiter, strip, quoted, segment in self.pending:
            lines = []
            while self.pos < size:
                end = text.find('\n', self.pos)
                end = size if end < 0 else end
                line = text[self.pos:end]
                self.pos = min(end + 1, size)
                if strip:
                    line = line.lstrip('\t')
                if line == delimiter:
                    break
                lines.append(line)     # Bash also accepts a body ended by end of text
            body = '\n'.join(lines)
            if not quoted:
                # An unquoted delimiter still runs $( ) and backquotes in the body.
                Lexer(body, self.found, self.nesting).expansions()
            segment.heredocs.append(body)
        self.pending = []

    def expansions(self):
        text = self.text
        while match := SPECIAL.search(text, self.pos):
            self.pos = match.start()
            if match.group() == '\\':
                self.pos += 2
            elif match.group() == '$':
                self.dollar(quoted=True)
            else:
                self.backquote()

    def word(self):
        """One word, quotes removed; expansions become DYNAMIC, their commands are lexed."""
        text, size, parts, start = self.text, len(self.text), [], self.pos
        while self.pos < size:
            if match := PLAIN.match(text, self.pos):
                parts.append(match.group())
                self.pos = match.end()
                if (text.startswith('(', self.pos) and parts[-1].endswith('=')
                        and ASSIGNMENT.fullmatch(''.join(parts))):
                    self.pos += 1
                    self.array()           # NAME=( ... ) holds words, not commands
                    parts.append(DYNAMIC)
                continue
            char = text[self.pos]
            if char == '\\':
                escaped = text[self.pos + 1:self.pos + 2]
                self.pos += 2
                if escaped != '\n':
                    parts.append(escaped)
            elif char == "'":
                end = text.find("'", self.pos + 1)
                if end < 0:
                    raise ShellError('unterminated single quote')
                parts.append(text[self.pos + 1:end])
                self.pos = end + 1
            elif char == '"':
                self.pos += 1
                parts.append(self.double_quoted())
            elif char == '$':
                parts.append(self.dollar(quoted=False))
            elif char == '`':
                parts.append(self.backquote())
            else:
                break
        if self.pos == start:
            raise ShellError(f'unexpected {text[start:start + 1]!r}')
        return ''.join(parts)

    def array(self):
        while True:
            self.blanks()
            if self.pos >= len(self.text):
                raise ShellError('unterminated array assignment')
            char = self.text[self.pos]
            if char == ')':
                self.pos += 1
                return
            if char == '\n':
                self.pos += 1
                self.heredocs()
            else:
                self.word()

    def double_quoted(self):
        text, size, parts = self.text, len(self.text), []
        while True:
            if match := PLAIN_DOUBLE.match(text, self.pos):
                parts.append(match.group())
                self.pos = match.end()
            if self.pos >= size:
                raise ShellError('unterminated double quote')
            char = text[self.pos]
            if char == '"':
                self.pos += 1
                return ''.join(parts)
            if char == '\\':
                escaped = text[self.pos + 1:self.pos + 2]
                self.pos += 2
                if escaped != '\n':
                    parts.append(escaped if escaped in '$`"\\' else '\\' + escaped)
            elif char == '$':
                parts.append(self.dollar(quoted=True))
            else:
                parts.append(self.backquote())

    def dollar(self, quoted):
        text, pos = self.text, self.pos
        following = text[pos + 1:pos + 2]
        if following == '(':
            if not text.startswith('((', pos + 1):
                self.pos = pos + 2
                return self.substitution()
            self.pos = pos + 3
            self.arithmetic()
            return DYNAMIC
        if following == '{':
            self.pos = pos + 2
            self.braced(quoted)
            return DYNAMIC
        if following == "'" and not quoted:
            match = ANSI_C.match(text, pos + 2)
            if not match:
                raise ShellError('unterminated ANSI-C quote')
            self.pos = match.end()
            return match.group()[:-1]
        if following == '"' and not quoted:
            self.pos = pos + 1          # $"..." is a translated "..."
            return ''
        if match := PARAMETER.match(text, pos + 1):
            self.pos = match.end()
            return DYNAMIC
        self.pos = pos + 1
        return '$'

    def braced(self, quoted):
        text, size, depth = self.text, len(self.text), 0
        while self.pos < size:
            char = text[self.pos]
            if char == '}':
                self.pos += 1
                if not depth:
                    return
                depth -= 1
            elif char == '{':
                depth += 1
                self.pos += 1
            elif char == '\\':
                self.pos += 2
            elif char == "'" and not quoted:
                end = text.find("'", self.pos + 1)
                if end < 0:
                    raise ShellError('unterminated single quote')
                self.pos = end + 1
            elif char == '"':
                self.pos += 1
                self.double_quoted()
            elif char == '$':
                self.dollar(quoted=True)
            elif char == '`':
                self.backquote()
            else:
                self.pos += 1
        raise ShellError('unterminated parameter expansion')

    def arithmetic(self):
        text, size, depth = self.text, len(self.text), 0
        while self.pos < size:
            char = text[self.pos]
            if char == ')':
                if depth:
                    depth -= 1
                    self.pos += 1
                elif text.startswith('))', self.pos):
                    self.pos += 2
                    return
                else:
                    raise ShellError('unsupported arithmetic expression')
            elif char == '(':
                depth += 1
                self.pos += 1
            elif char == '$':
                self.dollar(quoted=True)
            elif char == '`':
                self.backquote()
            else:
                self.pos += 1
        raise ShellError('unterminated arithmetic expression')

    def backquote(self):
        text, parts = self.text, []
        self.pos += 1
        while True:
            match = BACKQUOTE_END.search(text, self.pos)
            if not match:
                raise ShellError('unterminated backquote')
            parts.append(text[self.pos:match.start()])
            self.pos = match.end()
            if match.group() == '`':
                break
            escaped = text[self.pos:self.pos + 1]
            parts.append(escaped if escaped in '$`\\' else '\\' + escaped)
            self.pos += 1
        code = ''.join(parts)
        Lexer(code, self.found, self.nesting).commands()
        return DYNAMIC + code


def analyse(command, cwd=None):
    """(frozenset of LOKAL/BAU, first undecidable part or None) for one command."""
    ctx = Context(command, cwd)
    kinds = _kinds(command, 0, ctx)
    return frozenset(kinds), (ctx.errors[0] if ctx.errors else None)


def load_command(command, cwd=None):
    """True: visible load; False: none visible; ValueError: not decidable."""
    kinds, error = analyse(command, cwd)
    if kinds:
        return True
    if error:
        raise error
    return False


def _kinds(command, depth, ctx):
    if depth > MAX_DEPTH:
        raise ShellError('shell nesting exceeds classifier limit')
    found = []
    try:
        Lexer(command, found).commands()
    except ShellError as exc:
        ctx.note(exc)   # Bash runs the lines before a syntax error
    remember_written(found, ctx)
    kinds = set()
    for segment in found:
        try:
            kinds |= segment_load(segment.words, segment, ctx, depth)
        except ShellError as exc:
            ctx.note(exc)
    return kinds


def remember_written(found, ctx):
    """Files this command writes from a heredoc or here-string (`cat > x.sh <<EOF`,
    `tee x.sh <<EOF`): a later `bash x.sh` in the same call runs that text,
    and the file does not exist yet when the hook looks."""
    for segment in found:
        body = segment.heredocs + segment.strings
        if not body:
            continue
        targets = list(segment.outputs)
        if segment.words and os.path.basename(segment.words[0]) == 'tee':
            targets += [word for word in segment.words[1:] if not word.startswith('-')]
        for target in targets:
            if DYNAMIC not in target and target != '/dev/null':
                ctx.written[ctx.path(target)] = '\n'.join(body)


def tool_kind(tool, args):
    return BAU if tool == 'cargo-remote-fmt' or (tool == 'cargo' and args[:1] == ['remote']) else LOKAL


def segment_load(words, segment, ctx, depth):
    while words and (words[0] in RESERVED or words[0] == 'function'
                     or ASSIGNMENT.match(words[0])):
        words = words[2:] if words[0] == 'function' else words[1:]
    if not words or words[0] in {'for', 'select', 'case'}:
        return set()    # loop words and case subjects are data
    tool = os.path.basename(words[0])
    args = words[1:]
    if tool in WRAPPERS:
        inner = unwrap(tool, args)
        return segment_load(inner, segment, ctx, depth) if inner else set()
    if tool in LOAD_TOOLS:
        return set() if help_or_version(tool, args) else {tool_kind(tool, args)}
    if tool == 'git':
        # Read-only status/log/show calls stay quiet; worktree changes and pushes
        # also appeared in the reported interference incidents.
        while args and args[0].startswith('-'):
            args = args[2:] if args[0] in {'-C', '-c', '--git-dir', '--work-tree'} else args[1:]
        return {LOKAL} if args and (args[0] == 'push' or args[:2] in [
            ['worktree', 'add'], ['worktree', 'remove'],
        ]) else set()
    if tool == 'jq':
        return {LOKAL} if RECORDINGS.search(ctx.text) else set()
    if tool == 'eval':
        return _kinds(' '.join(args), depth + 1, ctx)
    if tool in {'source', '.'}:
        return script_load(args[0], 'shell', ctx, depth) if args else set()
    if tool == 'find':
        return find_load(args, segment, ctx, depth)
    if tool in SHELLS:
        return shell_load(args, segment, ctx, depth)
    if tool in CODE_RUNNERS:
        return code_load(args, segment, ctx, depth)
    kinds = program_load(words[0], ctx, TARGET_BIN)
    if kinds or '/' not in words[0]:
        return kinds
    return script_load(words[0], None, ctx, depth, missing_is_quiet=True)


def program_load(name, ctx, pattern=None):
    """The load of a program named by path: a tools/ script (what it runs), a
    target/ binary, a load tool by its installed path. A program named only at
    run time is undecidable; in this project "$B" is mostly a Rust test binary."""
    if TOOLS_PATH.search(name):
        return tools_kinds(name, ctx)
    if pattern and pattern.search(name):
        return {LOKAL}
    base = os.path.basename(name)
    if base in LOAD_TOOLS:
        return {BAU if base == 'cargo-remote-fmt' else LOKAL}
    if NODE_TOOL.search(name):
        return {LOKAL}
    if DYNAMIC in base:
        raise ShellError('program name is only known at run time')
    return set()


def read_script(path):
    """The text of a script file, at most SCRIPT_LIMIT bytes; OSError if absent."""
    with open(path, 'rb') as source:
        raw = source.read(SCRIPT_LIMIT + 1)
    if len(raw) > SCRIPT_LIMIT:
        raise ShellError('script too large to classify')
    return raw.decode('utf-8', errors='replace')


def tools_kinds(name, ctx):
    """A tools/ script loads this machine, and the build service if its text
    names `cargo remote` (tests-lokal.sh, streich-mutation.py). Unreadable, it
    could be either, so it counts for both."""
    if DYNAMIC in name:
        return {LOKAL, BAU}
    path = ctx.path(name)
    text = ctx.written.get(path)
    if text is None:
        try:
            text = read_script(path)
        except (OSError, ShellError):
            return {LOKAL, BAU}
    return {LOKAL, BAU} if REMOTE_LITERAL.search(text) else {LOKAL}


def script_load(name, language, ctx, depth, missing_is_quiet=False):
    """A script run by name: a tools/ path or a load tool decide at once, else
    the text this command wrote to it, else the file. `language` is 'shell',
    'code' or None (decided by the #! line, as the kernel would)."""
    kinds = program_load(name, ctx)
    if kinds:
        return kinds
    path = ctx.path(name)
    body = ctx.written.get(path)
    if body is None:
        try:
            with open(path, 'rb') as source:
                # A program path run directly is a script only with a #! line;
                # a binary is not read further, whatever its size.
                if language is None and source.read(2) != b'#!':
                    return set()
        except OSError:
            if missing_is_quiet:
                return set()    # a program path, not a script we were asked to run
            raise ShellError('script is neither written here nor readable')
        try:
            body = read_script(path)
        except OSError:
            raise ShellError('script is neither written here nor readable')
    if language is None:
        first = body.split('\n', 1)[0]
        language = 'code' if any(runner in first for runner in CODE_RUNNERS) else 'shell'
    if language == 'code':
        return code_kinds(body, ctx)
    return _kinds(body, depth + 1, ctx.child(body))


def unwrap(tool, args):
    """The command a wrapper starts; [] if it starts none."""
    values, long_values, positional = WRAPPERS[tool]
    index = 0
    while index < len(args):
        arg = args[index]
        if arg == '--':
            index += 1
            break
        if tool in {'env', 'sudo'} and ASSIGNMENT.match(arg):
            index += 1
        elif arg.startswith('--'):
            name = arg.split('=', 1)[0]
            if name in NO_COMMAND_LONG:
                return []
            if tool == 'env' and name == '--split-string':
                value = arg.split('=', 1)[1] if '=' in arg else ''.join(args[index + 1:index + 2])
                return value.split() + args[index + (1 if '=' in arg else 2):]
            index += 2 if name in long_values and '=' not in arg else 1
        elif arg.startswith('-') and len(arg) > 1:
            for position, letter in enumerate(arg[1:], 1):
                if letter in NO_COMMAND.get(tool, ''):
                    return []
                if letter in values:
                    attached = arg[position + 1:]
                    if tool == 'env' and letter == 'S':
                        rest = args[index + (1 if attached else 2):]
                        return (attached or ''.join(args[index + 1:index + 2])).split() + rest
                    if not attached:
                        index += 1    # the value is the next word
                    break
            index += 1
        else:
            break
    rest = args[index + positional:]
    if tool == 'flock' and rest[:1] in (['-c'], ['--command']):
        return ['sh', '-c', *rest[1:2]]    # `flock FILE -c 'CMD'` hands CMD to a shell
    return rest


def help_or_version(tool, args):
    """Help, version and command lists start no build, test or format run;
    neither do the queries of Cargo Remote v2."""
    before = args[:args.index('--')] if '--' in args else args
    if tool == 'cargo' and before[:1] == ['remote']:
        query = before[1:2] and before[1].split('=', 1)[0] in REMOTE_QUERY and '--' not in args
        return before[1:] in (['--help'], ['--version']) or bool(query)
    flags = [arg for arg in before if arg.startswith('-')]
    names = [arg for arg in before if not arg.startswith('-')]
    if not all(SUBCOMMAND.fullmatch(name) for name in names) or len(names) > 2:
        return False
    if not flags:
        return names[:1] == ['help'] and tool in {'cargo', 'npm', 'pnpm', 'yarn'}
    if names and tool in {'pnpm', 'yarn'}:
        return False    # both pass flags after a script name on to the script
    return len(flags) == 1 and flags[0] in HELP[tool]


def find_load(args, segment, ctx, depth):
    kinds = set()
    for index, arg in enumerate(args):
        if arg in {'-exec', '-execdir', '-ok', '-okdir'}:
            inner = []
            for word in args[index + 1:]:
                if word in {';', '+'}:
                    break
                inner.append(word)
            kinds |= segment_load(inner, segment, ctx, depth)
    return kinds


def stdin_code(segment):
    """Code an interpreter reads from stdin; undecidable if it comes from a pipe."""
    if segment.heredocs or segment.strings:
        return segment.heredocs + segment.strings
    if segment.piped:
        raise ShellError('interpreter reads code from a pipe')
    return []


def shell_load(args, segment, ctx, depth):
    index, reads_stdin = 0, False
    while index < len(args):
        arg = args[index]
        if arg in SHELL_VALUED:
            index += 2
            continue
        if arg in {'-', '--'}:
            index += 1
            break
        if not arg.startswith(('-', '+')):
            break
        if SHELL_CODE.fullmatch(arg):    # -c, -lc, -ec: the next word is the code
            return _kinds(args[index + 1], depth + 1, ctx) if index + 1 < len(args) else set()
        reads_stdin = reads_stdin or bool(SHELL_STDIN.fullmatch(arg))
        index += 1
    operands = args[index:]
    if operands and not reads_stdin:
        return script_load(operands[0], 'shell', ctx, depth)
    if segment.inputs and not (segment.heredocs or segment.strings):
        return set().union(*(script_load(name, 'shell', ctx, depth) for name in segment.inputs))
    return set().union(*(_kinds(code, depth + 1, ctx) for code in stdin_code(segment)))


def code_kinds(code, ctx):
    """Inline Python/Node code loads only if it visibly starts something, and
    then what it starts decides the kind."""
    if not STARTS.search(code):
        return set()
    kinds = set()
    for name in TOOLS_FILE.findall(code):
        kinds |= tools_kinds(name, ctx)
    if REMOTE_LITERAL.search(code):
        kinds.add(BAU)
    if LOCAL_LITERAL.search(code) or CODE_TARGET.search(code):
        kinds.add(LOKAL)
    return kinds


def code_load(args, segment, ctx, depth):
    """Python/Node: a tools/ script, a script that visibly starts load, or inline
    code that does."""
    index = 0
    while index < len(args):
        arg = args[index]
        if arg in CODE_VALUED:
            index += 2
        elif INLINE_CODE.fullmatch(arg):
            return code_kinds(args[index + 1], ctx) if index + 1 < len(args) else set()
        elif arg == '-m':
            return set()      # a module name, not a tools/ path
        elif arg == '--':
            index += 1
            break
        elif arg.startswith('-') and arg != '-':
            index += 1
        else:
            break
    operands = args[index:]
    if operands and operands[0] != '-':
        return script_load(operands[0], 'code', ctx, depth)
    if segment.inputs and not (segment.heredocs or segment.strings):
        return set().union(*(script_load(name, 'code', ctx, depth) for name in segment.inputs))
    return set().union(*(code_kinds(code, ctx) for code in stdin_code(segment)))
