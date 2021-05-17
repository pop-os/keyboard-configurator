import re
import json
from collections import OrderedDict

with open('layouts/picker.json') as f:
    picker = json.load(f)
picker_keycodes = [key['keysym'] for group in picker for key in group['keys']]

LINUX_MAPPING = {i.replace('NUM_', 'KP').replace('_', ''): i for i in picker_keycodes}
LINUX_MAPPING.update({
    'BACKSPACE': 'BKSP',
    'DOT': 'PERIOD',
    'KPDOT': 'NUM_PERIOD',
    'LEFTBRACE': 'BRACE_OPEN',
    'LEFTMETA': 'LEFT_SUPER',
    'RIGHTBRACE': 'BRACE_CLOSE',
    'RIGHTMETA': 'RIGHT_SUPER',
    'DELETE': 'DEL',
    'CAPSLOCK': 'CAPS',
    'EQUAL': 'EQUALS',
    'PAGEUP': 'PGUP',
    'PAGEDOWN': 'PGDN',
    'GRAVE': 'TICK', 
    'NEXTSONG': 'MEDIA_NEXT',
    'PREVIOUSSONG': 'MEDIA_PREV',
    'APOSTROPHE': 'QUOTE',
    'PRINT': 'PRINT_SCREEN',
    'MENU': 'APP',
    'NUMLOCK': 'NUM_LOCK',
})

WHITELIST = [
    'NONE',
    'ROLL_OVER',
    'RESET',
    'FAN_TOGGLE',
    'KBD_UP',
    'KBD_DOWN',
    'KBD_BKL',
    'KBD_COLOR',
    'KBD_TOGGLE',
    'LAYER_ACCESS_1',
    'LAYER_ACCESS_3',
    'LAYER_ACCESS_4',
    'LAYER_TOGGLE_1',
    'LAYER_TOGGLE_2',
    'LAYER_TOGGLE_3',
    'LAYER_TOGGLE_4',
]

hdr = open('/usr/include/linux/input-event-codes.h').read()
linux_keymap = re.findall('#define KEY_(\S*)\s*((?:0x)?[0-9]+)', hdr, re.MULTILINE)
linux_keymap = [(k, int(v, base=0)) for k, v in linux_keymap]
linux_keymap = OrderedDict((LINUX_MAPPING.get(k, k), v) for k, v in linux_keymap)
del linux_keymap['FN']

# print([i for i in linux_keymap.keys() if i not in picker_keycodes])
# print([i for i in picker_keycodes if i not in linux_keymap.keys() if i not in WHITELIST])

print(json.dumps(linux_keymap, indent=2))

