# Incompatibility List
The assembler should work if the compiler fails.

## FE9
- C11.cmb (found match opcodes outside of match context - needs investigation)
- C22.cmb (fails to disassemble due to empty expr stack - needs investigation)
- C28.cmb (fails to disassemble due to empty expr stack - needs investigation)
- Everything for the compiler. This is not being prioritized.

## FE10
- C0401.cmb (Exalt evaluates a constant expression that IS's did not)

## FE11
- command.cmb

## FE12
- bmap023.cmb (fails to decompile due to empty expr stack - needs investigation)
- bmap303.cmb (mismatch because the original file has strange padding)
- command.cmb

## FE13
- bev.cmb
    - Functionally equivalent, text is reordered
- bev_shared.cmb
    - Functionally equivalent, text is reordered
- Command.cmb
    - Functionally equivalent, text is reordered

## FE14 (Base Game)
- bev.cmb
    - Functionally equivalent, text is reordered
- Command.cmb
    - Functionally equivalent, text is reordered

## FE14 (DLC)
- Heirs of Fate 4 terrain script
    - This uses switch/case fall through which Exalt will not support.
- Museum Melee chapter script
    - Functionally equivalent, text is reordered

## FE15
- bev.cmb
    - Functionally equivalent, text is reordered
- Command.cmb
    - Functionally equivalent, text is reordered
- GMAP.cmb
    - Functionally equivalent, text is reordered
