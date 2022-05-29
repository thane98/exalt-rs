# Incompatibility List
The assembler should work if the compiler fails.

## FE9
- C11.cmb (found match opcodes outside of match context - needs investigation)
- C22.cmb (fails to decompile due to empty expr stack - needs investigation)
- C28.cmb (fails to decompile due to empty expr stack - needs investigation)

## FE10
- startup.cmb (fails to decompile - likely due to switch/case fall through)

## FE11

## FE12
- bmap023.cmb (fails to decompile due to empty expr stack - needs investigation)
- bmap303.cmb (mismatch because the original file has strange padding)

## FE13
- aDebug.cmb (pending feature)
- bev.cmb
- bev_shared.cmb
- Command.cmb

## FE14 (Base Game)
- bev.cmb
- Command.cmb

## FE14 (DLC)
- Heirs of Fate 4 terrain script
    - This uses switch/case fall through which Exalt will not support.
- Museum Melee chapter script
    - Functionally equivalent, text is reordered

## FE15
- bev.cmb
- Command.cmb
- GMAP.cmb
