#!/usr/bin/env python3
import curses
import os
import sys


ROOT_KCONFIG = "Kconfig"
DEFAULT_CONFIG = ".config"
AUTOCONF_MK = "include/generated/autoconf.mk"
AUTOCONF_RS = "include/generated/autoconf.rs"


class Symbol:
    def __init__(self, name):
        self.name = name
        self.kind = "bool"
        self.prompt = name
        self.default = "n"
        self.depends = []
        self.menu = "General"
        self.value = None

    def config_value(self):
        value = self.effective_value()
        if self.kind == "bool":
            return "y" if value in ("y", "true", "1") else "n"
        return str(value)

    def effective_value(self):
        if self.value is not None:
            return self.value
        return self.default

    def enabled(self, symbols):
        for dep in self.depends:
            symbol = symbols.get(dep)
            if symbol is None or symbol.config_value() != "y":
                return False
        return True


def parse_kconfig(path):
    symbols = []
    by_name = {}
    menu = "General"
    current = None

    with open(path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    for raw in lines:
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("mainmenu "):
            continue
        if line.startswith("menu "):
            menu = unquote(line[len("menu "):])
            continue
        if line == "endmenu":
            menu = "General"
            continue
        if line.startswith("config "):
            name = line.split(None, 1)[1]
            current = Symbol(name)
            current.menu = menu
            symbols.append(current)
            by_name[name] = current
            continue
        if current is None:
            continue

        if line.startswith("bool"):
            current.kind = "bool"
            current.prompt = parse_prompt(line, "bool", current.name)
        elif line.startswith("int"):
            current.kind = "int"
            current.prompt = parse_prompt(line, "int", current.name)
        elif line.startswith("hex"):
            current.kind = "hex"
            current.prompt = parse_prompt(line, "hex", current.name)
        elif line.startswith("string"):
            current.kind = "string"
            current.prompt = parse_prompt(line, "string", current.name)
        elif line.startswith("default "):
            current.default = unquote(line[len("default "):])
        elif line.startswith("depends on "):
            deps = line[len("depends on "):].replace("&&", " ").split()
            current.depends.extend(dep for dep in deps if dep not in ("(", ")"))

    return symbols, by_name


def parse_prompt(line, keyword, fallback):
    rest = line[len(keyword):].strip()
    if not rest:
        return fallback
    return unquote(rest)


def unquote(value):
    value = value.strip()
    if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
        return value[1:-1]
    return value


def load_config(path, symbols):
    if not os.path.exists(path):
        return
    by_name = {symbol.name: symbol for symbol in symbols}
    with open(path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line:
                continue
            if line.startswith("# CONFIG_") and line.endswith(" is not set"):
                name = line[len("# CONFIG_"):-len(" is not set")]
                symbol = by_name.get(name)
                if symbol is not None and symbol.kind == "bool":
                    symbol.value = "n"
                continue
            if line.startswith("#") or "=" not in line:
                continue
            key, value = line.split("=", 1)
            if not key.startswith("CONFIG_"):
                continue
            symbol = by_name.get(key[len("CONFIG_"):])
            if symbol is None:
                continue
            symbol.value = unquote(value)


def load_defconfig(path, symbols):
    load_config(path, symbols)


def write_outputs(config_path, symbols, by_name):
    os.makedirs(os.path.dirname(AUTOCONF_MK), exist_ok=True)
    for symbol in symbols:
        if not symbol.enabled(by_name):
            symbol.value = "n" if symbol.kind == "bool" else symbol.default

    with open(config_path, "w", encoding="utf-8") as f:
        for symbol in symbols:
            value = symbol.config_value()
            if symbol.kind == "bool":
                if value == "y":
                    f.write(f"CONFIG_{symbol.name}=y\n")
                else:
                    f.write(f"# CONFIG_{symbol.name} is not set\n")
            else:
                if symbol.kind == "string":
                    f.write(f'CONFIG_{symbol.name}="{value}"\n')
                else:
                    f.write(f"CONFIG_{symbol.name}={value}\n")

    with open(AUTOCONF_MK, "w", encoding="utf-8") as f:
        for symbol in symbols:
            value = symbol.config_value()
            if symbol.kind == "bool":
                f.write(f"CONFIG_{symbol.name}={value}\n")
            else:
                f.write(f"CONFIG_{symbol.name}:={value}\n")

    with open(AUTOCONF_RS, "w", encoding="utf-8") as f:
        for symbol in symbols:
            value = symbol.config_value()
            if symbol.kind == "bool":
                f.write(f"pub const CONFIG_{symbol.name}: bool = {str(value == 'y').lower()};\n")
            elif symbol.kind in ("int", "hex"):
                f.write(f"pub const CONFIG_{symbol.name}: usize = {value};\n")
            elif symbol.kind == "string":
                escaped = value.replace("\\", "\\\\").replace('"', '\\"')
                f.write(f'pub const CONFIG_{symbol.name}: &str = "{escaped}";\n')


def set_defaults(symbols):
    for symbol in symbols:
        symbol.value = symbol.default


def cmd_defconfig(args):
    symbols, by_name = parse_kconfig(ROOT_KCONFIG)
    set_defaults(symbols)
    defconfig = args[0] if args else "configs/qemu_virt_defconfig"
    load_defconfig(defconfig, symbols)
    write_outputs(DEFAULT_CONFIG, symbols, by_name)


def cmd_oldconfig(_args):
    symbols, by_name = parse_kconfig(ROOT_KCONFIG)
    set_defaults(symbols)
    load_config(DEFAULT_CONFIG, symbols)
    write_outputs(DEFAULT_CONFIG, symbols, by_name)


def cmd_menuconfig(_args):
    symbols, by_name = parse_kconfig(ROOT_KCONFIG)
    set_defaults(symbols)
    load_config(DEFAULT_CONFIG, symbols)
    curses.wrapper(menuconfig_ui, symbols, by_name)
    write_outputs(DEFAULT_CONFIG, symbols, by_name)


def menuconfig_ui(stdscr, symbols, by_name):
    curses.curs_set(0)
    index = 0
    while True:
        stdscr.erase()
        height, width = stdscr.getmaxyx()
        stdscr.addnstr(0, 0, "Rust AArch32 Kernel Configuration", width - 1, curses.A_BOLD)
        stdscr.addnstr(1, 0, "arrows/j/k: move  space: toggle  enter: edit  s: save  q: save+quit", width - 1)
        visible = [s for s in symbols if s.enabled(by_name) or s.config_value() == "y"]
        if index >= len(visible):
            index = max(0, len(visible) - 1)
        top = max(0, index - (height - 5))
        last_menu = None
        row = 3
        for pos, symbol in enumerate(visible[top:top + height - 4], start=top):
            if symbol.menu != last_menu and row < height - 1:
                stdscr.addnstr(row, 0, f"[{symbol.menu}]", width - 1, curses.A_BOLD)
                row += 1
                last_menu = symbol.menu
            if row >= height - 1:
                break
            marker = marker_for(symbol)
            text = f"{marker} {symbol.prompt} ({symbol.name})"
            attr = curses.A_REVERSE if pos == index else curses.A_NORMAL
            stdscr.addnstr(row, 0, text, width - 1, attr)
            row += 1
        key = stdscr.getch()
        if key in (ord("q"), ord("s")):
            return
        if key in (curses.KEY_UP, ord("k")):
            index = max(0, index - 1)
        elif key in (curses.KEY_DOWN, ord("j")):
            index = min(len(visible) - 1, index + 1)
        elif key == ord(" "):
            toggle(visible[index])
        elif key in (curses.KEY_ENTER, 10, 13):
            edit_value(stdscr, visible[index])


def marker_for(symbol):
    value = symbol.config_value()
    if symbol.kind == "bool":
        return "[*]" if value == "y" else "[ ]"
    return f"({value})"


def toggle(symbol):
    if symbol.kind == "bool":
        symbol.value = "n" if symbol.config_value() == "y" else "y"


def edit_value(stdscr, symbol):
    if symbol.kind == "bool":
        toggle(symbol)
        return
    curses.echo()
    height, width = stdscr.getmaxyx()
    prompt = f"{symbol.name}={symbol.config_value()} -> "
    stdscr.addnstr(height - 1, 0, " " * (width - 1), width - 1)
    stdscr.addnstr(height - 1, 0, prompt, width - 1)
    stdscr.refresh()
    data = stdscr.getstr(height - 1, min(len(prompt), width - 2), 128)
    curses.noecho()
    value = data.decode("utf-8").strip()
    if value:
        symbol.value = value


def main():
    if len(sys.argv) < 2:
        print("usage: scripts/kconfig.py <defconfig|oldconfig|menuconfig> [defconfig]", file=sys.stderr)
        return 2
    command = sys.argv[1]
    args = sys.argv[2:]
    if command == "defconfig":
        cmd_defconfig(args)
    elif command == "oldconfig":
        cmd_oldconfig(args)
    elif command == "menuconfig":
        cmd_menuconfig(args)
    else:
        print(f"unknown kconfig command: {command}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
