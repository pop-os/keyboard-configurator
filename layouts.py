#!/usr/bin/env python3

import argparse
from collections import OrderedDict
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import typing
from typing import List, Tuple, Dict

EXCLUDED_SCANCODES = ['INT_1', 'INT_2']
QMK_MAPPING = {
    'APPLICATION': 'APP',
    'AUDIO_MUTE': 'MUTE',
    'AUDIO_MUTE': 'MUTE',
    'AUDIO_VOL_DOWN': 'VOLUME_DOWN',
    'AUDIO_VOL_UP': 'VOLUME_UP',
    'AGIN': 'AGAIN',
    'VOLD': 'VOLUME_DOWN',
    'VOLU': 'VOLUME_UP',
    'BSLASH': 'BACKSLASH',
    'BSLS': 'BACKSLASH',
    'SLSH': 'SLASH',
    'BSPACE': 'BKSP',
    'BACKSPACE': 'BKSP',
    'BSPC': 'BKSP',
    'BOOT': 'RESET',
    'BOOTLOADER': 'RESET',
    'SPC': 'SPACE',
    'SLCT': 'SELECT',
    'CAPSLOCK': 'CAPS',
    'CAPS_LOCK': 'CAPS',
    'LCAP': 'LOCKING_CAPS_LOCK',
    'LEAD': 'LEADER',
    'DELETE': 'DEL',
    'DOT': 'PERIOD',
    'EQUAL': 'EQUALS',
    'EQL': 'EQUALS',
    'ESCAPE': 'ESC',
    'EXSL': 'EXSEL',
    'GRAVE': 'TICK',
    'GESC': 'GRAVE_ESCAPE',
    'GRV': 'TICK',
    'KP_0': 'NUM_0',
    'KP_1': 'NUM_1',
    'KP_2': 'NUM_2',
    'KP_3': 'NUM_3',
    'KP_4': 'NUM_4',
    'KP_5': 'NUM_5',
    'KP_6': 'NUM_6',
    'KP_7': 'NUM_7',
    'KP_8': 'NUM_8',
    'KP_9': 'NUM_9',
    'P0': 'NUM_0',
    'P1': 'NUM_1',
    'P2': 'NUM_2',
    'P3': 'NUM_3',
    'P4': 'NUM_4',
    'P5': 'NUM_5',
    'P6': 'NUM_6',
    'P7': 'NUM_7',
    'P8': 'NUM_8',
    'P9': 'NUM_9',
    'KP_ASTERISK': 'NUM_ASTERISK',
    'PAST': 'NUM_ASTERISK',
    'INS': 'INSERT',
    'KP_COMMA': 'NUM_COMMA',
    'PCMM': 'NUM_COMMA',
    'COMM': 'COMMA',
    'CNCL': 'CANCEL',
    'CLR': 'CLEAR',
    'CLAG': 'CLEAR_AGAIN',
    'CRSL': 'CRSEL',
    'CALC': 'CALCULATOR',
    'KP_DOT': 'NUM_PERIOD',
    'PDOT': 'NUM_PERIOD',
    'KP_ENTER': 'NUM_ENTER',
    'ENT': 'ENTER',
    'EXEC': 'EXECUTE',
    'PENT': 'NUM_ENTER',
    'KP_EQUAL': 'NUM_EQUALS',
    'PEQL': 'NUM_EQUALS',
    'KP_MINUS': 'NUM_MINUS',
    'PMNS': 'NUM_MINUS',
    'MINS': 'MINUS',
    'KP_PLUS': 'NUM_PLUS',
    'PPLS': 'NUM_PLUS',
    'KP_SLASH': 'NUM_SLASH',
    'PSLS': 'NUM_SLASH',
    'LALT': 'LEFT_ALT',
    'ALGL': 'LEFT_ALT',
    'LOPT': 'LEFT_ALT',
    'LBRACKET': 'BRACE_OPEN',
    'LEFT_BRACKET': 'BRACE_OPEN',
    'LBRC': 'BRACE_OPEN',
    'LCTRL': 'LEFT_CTRL',
    'LCTL': 'LEFT_CTRL',
    'LGUI': 'LEFT_SUPER',
    'LWIN': 'LEFT_SUPER',
    'LCMD': 'LEFT_SUPER',
    'LEFT_GUI': 'LEFT_SUPER',
    'LSHIFT': 'LEFT_SHIFT',
    'LSFT': 'LEFT_SHIFT',
    'NO': 'NONE',
    'BASIC': 'NONE',
    'MEDIA_NEXT_TRACK': 'MEDIA_NEXT',
    'MNXT': 'MEDIA_NEXT',
    'MEDIA_PLAY_PAUSE': 'PLAY_PAUSE',
    'MPLY': 'PLAY_PAUSE',
    'MFFD': 'MEDIA_FAST_FORWARD',
    'MRWD': 'MEDIA_REWIND',
    'PAUS': 'PAUSE',
    'BRK': 'PAUSE',
    'BRMU': 'PAUSE',
    'ERAS': 'ALTERNATE_ERASE',
    'ASST': 'ASSISTANT',
    'MEDIA_PREV_TRACK': 'MEDIA_PREV',
    'MPRV': 'MEDIA_PREV',
    'MSTP': 'MEDIA_STOP',
    'MSEL': 'MEDIA_SELECT',
    'MS_U': 'MS_UP',
    'MS_D': 'MS_DOWN',
    'MS_L': 'MS_LEFT',
    'MS_R': 'MS_RIGHT',
    'EJCT': 'MEDIA_EJECT',
    'MYCM': 'MY_COMPUTER',
    'COMPUTER': 'MY_COMPUTER',
    'CPNL': 'CONTROL_PANEL',
    'WSCH': 'WWW_SEARCH',
    'WHOM': 'WWW_HOME',
    'WBAK': 'WWW_BACK',
    'WFWD': 'WWW_FORWARD',
    'WSTP': 'WWW_STOP',
    'WREF': 'WWW_REFRESH',
    'WFAV': 'WWW_FAVORITES',
    'WH_U': 'MS_WH_UP',
    'WH_D': 'MS_WH_DOWN',
    'WH_L': 'MS_WH_LEFT',
    'WH_R': 'MS_WH_RIGHT',
    'ACL0': 'MS_ACCEL0',
    'ACL1': 'MS_ACCEL1',
    'ACL2': 'MS_ACCEL2',
    'NUMLOCK': 'NUM_LOCK',
    'NUM': 'NUM_LOCK',
    'LNUM': 'LOCKING_NUM_LOCK',
    'NUHS': 'NONUS_HASH',
    'NUBS': 'NONUS_BACKSLASH',
    'PGDOWN': 'PGDN',
    'PAGE_DOWN': 'PGDN',
    'PAGE_UP': 'PGUP',
    'PSCREEN': 'PRINT_SCREEN',
    'PSCR': 'PRINT_SCREEN',
    'PRIR': 'PRIOR',
    'PWR': 'SYSTEM_POWER',
    'INTERNATIONAL_1': 'INT1',
    'INTERNATIONAL_2': 'INT2',
    'INTERNATIONAL_3': 'INT3',
    'INTERNATIONAL_4': 'INT4',
    'INTERNATIONAL_5': 'INT5',
    'INTERNATIONAL_6': 'INT6',
    'INTERNATIONAL_7': 'INT7',
    'INTERNATIONAL_8': 'INT8',
    'INTERNATIONAL_9': 'INT9',
    'LNG1': 'LANGUAGE_1',
    'LNG2': 'LANGUAGE_2',
    'LNG3': 'LANGUAGE_3',
    'LNG4': 'LANGUAGE_4',
    'LNG5': 'LANGUAGE_5',
    'LNG6': 'LANGUAGE_6',
    'LNG7': 'LANGUAGE_7',
    'LNG8': 'LANGUAGE_8',
    'LNG9': 'LANGUAGE_9',
    'PSTE': 'PASTE',
    'PROGRAMMABLE_BUTTON_1': 'PROGRAMMABLE_BUTTON',
    'QUOT': 'QUOTE',
    'QUANTUM': 'RESET',
    'RALT': 'RIGHT_ALT',
    'ALGR': 'RIGHT_ALT',
    'ROPT': 'RIGHT_ALT',
    'RBRACKET': 'BRACE_CLOSE',
    'RIGHT_BRACKET': 'BRACE_CLOSE',
    'RBRC': 'BRACE_CLOSE',
    'RCTRL': 'RIGHT_CTRL',
    'RCTL': 'RIGHT_CTRL',
    'RGB_TOG': 'KBD_TOGGLE',
    'RGB_VAD': 'KBD_DOWN',
    'RGB_VAI': 'KBD_UP',
    'RGB_MOD': 'RGB_MODE_FORWARD',
    'RGB_RMOD': 'RGB_MODE_REVERSE',
    'RGB_M_P': 'RGB_MODE_PLAIN',
    'RGB_M_B': 'RGB_MODE_BREATHE',
    'RGB_M_R': 'RGB_MODE_RAINBOW',
    'RGB_M_SW': 'RGB_MODE_SWIRL',
    'RGB_M_SN': 'RGB_MODE_SNAKE',
    'RGB_M_K': 'RGB_MODE_KNIGHT',
    'RGB_M_X': 'RGB_MODE_XMAS',
    'RGB_M_G': 'RGB_MODE_GRADIENT',
    'RGB_M_T': 'RGB_MODE_RGBTEST',
    'RGB_MODE_TEST': 'RGB_MODE_RGBTEST',
    'RGB_M_TW': 'RGB_MODE_TWINKLE',
    'LIGHTING': 'BACKLIGHT_ON',
    'BRIU': 'BRIGHTNESS_UP',
    'BRID': 'BRIGHTNESS_DOWN',
    'RGHT': 'RIGHT',
    'RGUI': 'RIGHT_SUPER',
    'RWIN': 'RIGHT_SUPER',
    'RCMD': 'RIGHT_SUPER',
    'RIGHT_GUI': 'RIGHT_SUPER',
    'RSHIFT': 'RIGHT_SHIFT',
    'RSFT': 'RIGHT_SHIFT',
    'RETN': 'RETURN',
    'RBT': 'REBOOT',
    'SCOLON': 'SEMICOLON',
    'SCLN': 'SEMICOLON',
    'SCROLLLOCK': 'SCROLL_LOCK',
    'LSCR': 'LOCKING_SCROLL_LOCK',
    'SCRL': 'SCROLL_LOCK',
    'BRMD': 'SCROLL_LOCK',
    'JOYSTICK_BUTTON_0': 'JOYSTICK',
    'BTN1': 'MS_BTN1',
    'BTN2': 'MS_BTN2',
    'BTN3': 'MS_BTN3',
    'BTN4': 'MS_BTN4',
    'BTN5': 'MS_BTN5',
    'BTN6': 'MS_BTN6',
    'BTN7': 'MS_BTN7',
    'BTN8': 'MS_BTN8',
    'MIDI': 'MIDI_ON',
    'AUDIO': 'AUDIO_ON',
    'MACRO': 'MACRO_0',
    'SYSTEM_SLEEP': 'SUSPEND',
    'SLEP': 'SUSPEND',
    'SYRQ': 'SYSTEM_REQUEST',
    'WAKE': 'SYSTEM_WAKE',
    'SEPR': 'SEPARATOR',
    'TRANSPARENT': 'ROLL_OVER',
    'TRNS': 'ROLL_OVER',
    'TG(0)': 'LAYER_TOGGLE_1',
    'TOGGLE_LAYER': 'LAYER_TOGGLE_1',
    'TG(1)': 'LAYER_TOGGLE_2',
    'TG(2)': 'LAYER_TOGGLE_3',
    'TG(3)': 'LAYER_TOGGLE_4',
    'TO(0)': 'LAYER_SWITCH_1',
    'TO':    'LAYER_SWITCH_1',
    'TO(1)': 'LAYER_SWITCH_2',
    'TO(2)': 'LAYER_SWITCH_3',
    'TO(3)': 'LAYER_SWITCH_4',
    'MO(0)': 'LAYER_ACCESS_1',
    'MOMENTARY': 'LAYER_ACCESS_1',
    'MO(1)': 'FN',
    'MO(2)': 'LAYER_ACCESS_3',
    'MO(3)': 'LAYER_ACCESS_4',
    '_______': 'ROLL_OVER',
}
QMK_EXTRA_SCANCODES = [
    "TG(0)",
    "TG(1)",
    "TG(2)",
    "TG(3)",
    "TO(0)",
    "TO(1)",
    "TO(2)",
    "TO(3)",
    "MO(0)",
    "MO(1)",
    "MO(2)",
    "MO(3)",
]
EXCLUDE_BOARDS = [
    'system76/ortho_split_2u',
    'system76/launch_test',
    'system76/virgo',
]

ALIAS_RE = '#define\s+KC_([A-Z_]*)\s+KC_([A-Z_]+]*)\s*$'

# keycode_h = open('tmk_core/common/keycode.h').read()
# [(i.group(1), i.group(2)) for i in (re.match('#define\s+KC_([A-Z_]*)\s+KC_([A-Z_]+]*)\s*$', i) for i in keycode_h.splitlines()) if i]

def call_preprocessor(input: str) -> str:
    return subprocess.check_output(["gcc", "-E", "-nostdinc", "-"], stderr=subprocess.DEVNULL, input=input, universal_newlines=True)

def read_stripping_includes(path: str) -> str:
    output = ''
    with open(path) as f:
        for line in f:
            if not line.startswith('#include'):
                output += line
    return output

def extract_scancodes(ecdir: str, is_qmk: bool) -> Tuple[typing.OrderedDict[str, int], Dict[str, str]]:
    "Extract mapping from scancode names to numbers"

    is_old_qmk = False
    if is_qmk:
        version = subprocess.check_output(["git", "-C", ecdir, "describe", "--tags"], stderr=subprocess.DEVNULL, universal_newlines=True)
        is_old_qmk = "0.7.103" in version or "0.7.104" in version or "0.12.20" in version
        if is_old_qmk:
            include_paths = [f"{ecdir}/tmk_core/common/keycode.h", f"{ecdir}/quantum/quantum_keycodes.h", f"{ecdir}/tmk_core/common/action_code.h"]
            includes = [read_stripping_includes(i) for i in include_paths]
            common_keymap_h = call_preprocessor(includes[0])
            quantum_keycode_h = call_preprocessor(includes[1])
            scancode_defines = re.findall(
                '    (KC_[^,\s]+)', common_keymap_h)
            scancode_defines += re.findall(
                '    (RGB_[^,\s]+)', quantum_keycode_h)
        else:
            include_paths = [ f"{ecdir}/quantum/keycodes.h", f"{ecdir}/quantum/quantum_keycodes.h", f"{ecdir}/quantum/action_code.h" ]
            includes = [read_stripping_includes(i) for i in include_paths]
            keycodes_h = call_preprocessor(includes[0])
            quantum_keycode_h = call_preprocessor(includes[1])
            scancode_defines = re.findall(
                '    (KC_[^,\s]+)', keycodes_h)
            scancode_defines += re.findall(
                '    (RGB_[^,\s]+)', keycodes_h)
            scancode_defines += re.findall(
                '    (QK_[^,\s]+)', keycodes_h)
            scancode_defines += re.findall(
                '    (QK_[^,\s]+)', quantum_keycode_h)
        define_aliases = [(i.group(1), i.group(2)) for i in (re.match(ALIAS_RE, i) for i in includes[0].splitlines()) if i]
        mapping = QMK_MAPPING
        mapping.update({alias: QMK_MAPPING.get(keycode, keycode) for alias, keycode in define_aliases})
        for (alias, keycode) in define_aliases:
            mapping[alias] = QMK_MAPPING.get(keycode, keycode)
        scancode_defines += QMK_EXTRA_SCANCODES
    else:
        includes = [open(f"{ecdir}/src/common/include/common/keymap.h").read()]
        common_keymap_h = includes[0]
        scancode_defines = re.findall(
            '#define.*((?:K_\S+)|(?:KT_FN))', common_keymap_h)
        mapping = {}

    tmpdir = tempfile.mkdtemp()
    with open(f'{tmpdir}/keysym-extract.c', 'w') as f:
        f.write('#include <stdint.h>\n')
        for i in includes:
            f.write(i.replace('#pragma once', ''))
            f.write('\n')
        f.write('#include <stdio.h>\n')
        f.write('int main() {\n')
        for i in scancode_defines:
            f.write(f'printf("%d ", {i});\n')
        f.write('}\n')

    cmd = ['gcc']
    cmd += ['-o', f'{tmpdir}/keysym-extract', f'{tmpdir}/keysym-extract.c']
    subprocess.check_call(cmd)

    output = subprocess.check_output(
        f'{tmpdir}/keysym-extract', universal_newlines=True)
    scancodes = (int(i) for i in output.split())

    shutil.rmtree(tmpdir)

    scancode_names = []
    for name in scancode_defines:
        if '_' in name and name.split('_')[0] != 'RGB':
            name = name.split('_', 1)[1]
        if is_qmk:
            name = mapping.get(name, name)
        scancode_names.append(name)
    scancode_list = OrderedDict(zip(scancode_names, scancodes))

    if is_qmk:
        if is_old_qmk:
            scancode_list['RESET'] = 0x5C00
    else:
        scancode_list['NONE'] = 0x0000

    scancode_list = OrderedDict((name, code) for (name, code) in scancode_list.items() if name not in EXCLUDED_SCANCODES)

    # Make sure scancodes are unique
    assert len(scancode_list.keys()) == len(set(scancode_list.values()))

    return scancode_list, mapping


def parse_layout_define(keymap_h: str, is_qmk) -> Tuple[List[str], List[List[str]]]:
    keymap_h = re.sub(r'/\*.*?\*/', '', keymap_h)
    # XXX split up regex?
    m = re.search(
        r'LAYOUT\((.*?)\)[\s\\]*({[^{}]*({[^{}]*}[^{}]*)+)[^{}]*}', keymap_h, re.MULTILINE | re.DOTALL)
    assert m is not None
    physical = m.group(1).replace(',', ' ').replace('\\', '').split()
    # XXX name?
    physical2 = [i.replace('\\', '').replace(',', '').split()
                 for i in m.group(2).replace('{', '').split('}')[:-1]]
    assert is_qmk or all(len(i) == len(physical2[0]) for i in physical2)
    return physical, physical2

def parse_led_config(led_c: str, physical2: List[List[str]]) -> Dict[str, List[int]]:
    led_c = re.sub(r'//.*', '', led_c)
    led_c = re.sub(r'/\*.*?\*/', '', led_c)
    m = re.search(r'g_led_config.*{ \{(.*?)^},', led_c, re.MULTILINE | re.DOTALL)
    leds: Dict[str, List[int]] = {}
    if m is None:
        return leds
    for i, line in enumerate(re.findall(r'{(.*)?}', m.group(1))):
        for j, led_index in enumerate(line.replace(',', ' ').split()):
            if led_index != '__':
                leds[physical2[i][j]] = [int(led_index)]
    return leds

def parse_keymap(keymap_c: str, mapping: Dict[str, str], physical: List[str], is_qmk: bool) -> Dict[str, List[str]]:
    # XXX for launch
    keymap_c = keymap_c.replace('MO(1)', 'FN')
    keymap_c = re.sub(r'/\*.*?\*/', '', keymap_c)

    layer_scancodes: List[List[str]] = []
    for layer in re.finditer(r'LAYOUT\((.*?)\)', keymap_c, re.MULTILINE | re.DOTALL):
        scancodes = layer.group(1).replace(',', ' ').split()
        assert len(scancodes) == len(physical)

        def scancode_map(x: int, code: str) -> str:
            if code == '0':
                return 'NONE'

            code = code.replace('QK_', '').replace('K_', '').replace('KC_', '').replace('KT_', '')

            if is_qmk:
                code = mapping.get(code, code)

            return code

        scancodes = [scancode_map(x, i) for x, i in enumerate(scancodes)]
        layer_scancodes.append(scancodes)

    keymap = {}
    for i, physical_name in enumerate(physical):
        keymap[physical_name] = [j[i] for j in layer_scancodes]

    return keymap


def gen_layout_json(path: str, physical: List[str], physical2: List[List[str]]) -> None:
    "Generate layout.json file"

    layout = {}
    for p in physical:
        x, y = next((x, y) for x, i in enumerate(physical2)
                    for y, j in enumerate(i) if j == p)
        layout[p] = (x, y)

    write_json_file(path, layout)

def gen_keymap_json(path: str, scancodes: typing.OrderedDict[str, int]) -> None:
    "Generate keymap.json file"

    write_json_file(path, scancodes)

def gen_leds_json(path: str, leds: [str, List[int]]) -> None:
    "Generate leds.json file"

    write_json_file(path, leds, sort_keys=True)

def gen_default_json(path: str, board: str, keymap: Dict[str, List[str]], is_qmk: bool) -> None:
    "Generate default.json file"

    if is_qmk:
        key_leds = {k: None for k in keymap.keys()}
        layers = [
            {"mode": (7, 127), "brightness": 176, "color": (142, 255)},
            {"mode": (13, 127), "brightness": 176, "color": (142, 255)},
            {"mode": (13, 127), "brightness": 176, "color": (142, 255)},
            {"mode": (13, 127), "brightness": 176, "color": (142, 255)},
        ]
    else:
        key_leds = {}
        layers = [{"mode": None, "brightness": 0, "color": (0, 0)}]

    write_json_file(path, {"model": board, "version": 1, "map": keymap, "key_leds": key_leds, "layers": layers})


def update_meta_json(meta_json: str, has_brightness: bool, has_color: bool, keyboard: str):
    meta = {}
    if os.path.exists(meta_json):
        with open(meta_json, 'r') as f:
            meta = json.load(f, object_pairs_hook=OrderedDict)

    meta['has_brightness'] = has_brightness
    meta['has_color'] = has_color
    meta['keyboard'] = keyboard

    write_json_file(meta_json, meta)


def write_json_file(path: str, data, sort_keys=False):
    with open(path, 'w') as f:
        json.dump(data, f, indent=2, sort_keys=sort_keys)
        f.write('\n')


def generate_layout_dir(ecdir: str, board: str, is_qmk: bool) -> None:
    print(f'Generating layouts/{board}...')

    has_brightness = True
    has_color = True

    if is_qmk:
        keymap_h = open(
            f"{ecdir}/keyboards/{board}/{board.split('/')[-1]}.h").read()
        default_c = open(
            f"{ecdir}/keyboards/{board}/keymaps/default/keymap.c").read()
        led_c = open(
            f"{ecdir}/keyboards/{board}/{board.split('/')[-1]}.c").read()
        keyboard = board
    else:
        with open(f"{ecdir}/src/board/{board}/board.mk") as f:
            board_mk = f.read()

        m = re.search('^KEYBOARD=(.*)$', board_mk, re.MULTILINE)
        assert m is not None
        keyboard = board.rsplit('/', 1)[0] + '/' + m.group(1)

        m = re.search('^KBLED=(.*)$', board_mk, re.MULTILINE)
        assert m is not None
        kbled = m.group(1)
        # darp9 uses `rgb_pwm` but has keyboard with single-color backlight installed
        if kbled == 'white_dac' or board == 'system76/darp9':
            has_color = False
        # bonw14/bonw15: Handled through USB. Can configurator support this?
        elif kbled in ['none', 'bonw14', 'bonw15']:
            has_brightness = False
            has_color = False
        elif kbled not in ['rgb_pwm', 'oryp5', 'darp5']:
            raise Exception(f"KBLED='{kbled}' not handled by layouts.py")

        keymap_h = open(
            f"{ecdir}/src/keyboard/{keyboard}/include/board/keymap.h").read()
        default_c = open(f"{ecdir}/src/keyboard/{keyboard}/keymap/default.c").read()
        led_c = ""

    os.makedirs(f'layouts/{board}', exist_ok=True)
    os.makedirs(f'layouts/keyboards/{keyboard}', exist_ok=True)

    physical, physical2 = parse_layout_define(keymap_h, is_qmk)
    leds = parse_led_config(led_c, physical2)
    _scancodes, mapping = extract_scancodes(ecdir, is_qmk)
    default_keymap = parse_keymap(default_c, mapping, physical, is_qmk)
    gen_layout_json(f'layouts/keyboards/{keyboard}/layout.json', physical, physical2)
    gen_leds_json(f'layouts/keyboards/{keyboard}/leds.json', leds)
    gen_default_json(f'layouts/{board}/default.json', board, default_keymap, is_qmk)
    update_meta_json(f'layouts/{board}/meta.json', has_brightness, has_color, keyboard)

parser = argparse.ArgumentParser(usage="./layouts.py --qmk ../qmk_firmware system76/launch_heavy_1")
parser.add_argument("ecdir", help='For QMK boards that is the qmk_firmware (github.com/system76/qmk_firmware) directory itself, otherwise use the ec directory (github.com/system76/ec)')
parser.add_argument("board", help='The name of the manufacturer and board name. Example: "system76/launch_2"')
parser.add_argument("--qmk", action="store_true", help="Required if you plan on using a keyboard with QMK firmware.")
parser.add_argument("--qmk-legacy", action="store_true", help="Re-generate keymap json for old qmk version. (pre 0.19)")
args = parser.parse_args()

# Generate keymap file, used for all ec or qmk boards
scancodes, _mapping = extract_scancodes(args.ecdir, args.qmk or args.qmk_legacy)
if args.qmk_legacy:
    keymap_path = 'layouts/keymap/qmk_legacy.json'
elif args.qmk:
    keymap_path = 'layouts/keymap/qmk.json'
else:
    keymap_path = 'layouts/keymap/ec.json'
gen_keymap_json(keymap_path, scancodes)

if args.qmk_legacy:
    # Only keymap differs for legacy qmk, so don't generate anything related to individial boards
    sys.exit(0)

if args.board == 'all':
    if args.qmk:
        boarddir = f'{args.ecdir}/keyboards/system76'
    else:
        boarddir = f'{args.ecdir}/src/board/system76'
    for i in os.listdir(boarddir):
        if i == 'common' or not os.path.isdir(f'{boarddir}/{i}') or f"system76/{i}" in EXCLUDE_BOARDS:
            continue
        generate_layout_dir(args.ecdir, f'system76/{i}', args.qmk)
else:
    generate_layout_dir(args.ecdir, args.board, args.qmk)


