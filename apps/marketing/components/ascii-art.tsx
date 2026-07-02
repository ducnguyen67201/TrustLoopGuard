/* Generated ASCII assets — vaultandzn-style character art (F # / : . ramp).
   Decorative only: every instance renders aria-hidden with pointer-events off. */

export const ASCII_ART = {
  shield: `
 .  .    ..     . . . ..   .     . .         . .
................................................
. ..########################################...
. ..##FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF##.. .
  ..##FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF##...
. ..##FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF##...
 ...##FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF##...
. ..##FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF##..
 ...##FFFFFFFFFFFFFFFFFFFFFFFFFFF   FFFFFF##..
  ..##FFFFFFFFFFFFFFFFFFFFFFFFFF    FFFFFF##...
 ...##FFFFFFFFFFFFFFFFFFFFFFFF      FFFFFF##..
....##FFFFFFFFFFFFFFFFFFFFFFF       FFFFFF##.. .
 ...##FFFFFFFF   FFFFFFFFFF       FFFFFFFF##.. .
  ..###FFFFFFF    FFFFFFFF       FFFFFFFF###...
. ...##FFFFFFF      FFFF       FFFFFFFFFF##...
   ..##FFFFFFF       FF       FFFFFFFFFFF##..
. ...##FFFFFFFFF            FFFFFFFFFFFFF##.....
 .  ..##FFFFFFFFF          FFFFFFFFFFFFF##..
   ....##FFFFFFFFFF      FFFFFFFFFFFFFF##......
     ..##FFFFFFFFFFF    FFFFFFFFFFFFFFF##..   .
     ...##FFFFFFFFFFF  FFFFFFFFFFFFFFF##.. .
   .  ...##FFFFFFFFFFFFFFFFFFFFFFFFFF##...
        ..###FFFFFFFFFFFFFFFFFFFFFF###.. .
        ....##FFFFFFFFFFFFFFFFFFFF##...
       ... ..##FFFFFFFFFFFFFFFFFF##...  .
          .....##FFFFFFFFFFFFFF##... .
          . . ..##FFFFFFFFFFFF##..
             .  ..##FFFFFFFF##..   .
              ... ..##FFFF##..   .
                .   ..####... .`,
  globe: `
                  . . ..   .     .
             ..  ...FFFFFFFFFFFF...   .
             ..FFFFFFFFFFFFFFFFFFFFFF..
          ..FFFFFFFFFFFFFFFFFFFFFFFFFFFF..
        ..FFFFFF  FFF   FFFF   FFF  FFFFFF...
     . .FFF FFF  FFF    FFFF    FFF  FFF FFF...
    ...FF  FF   FFF     FFFF     FFF   FF  FF..
   ..FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF.
   .FF   FFF    FF      FFFF      FF    FFF   FF.
  ..FF   FF    FFF      FFFF      FFF    FF   FF..
 ..FF   FFF    FFF      FFFF      FFF    FFF   FF. .
...F    FF     FF       FFFF       FF     FF    F..
 .FF    FF     FF       FFFF       FF     FF    FF.
 .FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF.
..FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF.
 .FF    FF     FF       FFFF       FF     FF    FF.
...F    FF     FF       FFFF       FF     FF    F..
 ..FF   FFF    FFF      FFFF      FFF    FFF   FF. .
  ..FF   FF    FFF      FFFF      FFF    FF   FF...
   .FF   FFF    FF      FFFF      FF    FFF   FF..
    .FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF..
   . ..FF  FF   FFF     FFFF     FFF   FF  FF...
       .FFF FFF  FFF    FFFF    FFF  FFF FFF. .
        ..FFFFFF  FFF   FFFF   FFF  FFFFFF...
        ....FFFFFFFFFFFFFFFFFFFFFFFFFFFF..
          .. ..FFFFFFFFFFFFFFFFFFFFFF.. .
                ....FFFFFFFFFFFF...   .
                  .  .   .`,
  padlock: `
             ........ .
          ...##########.. .
         .##F###....###F##..
     .  .##F#..     ...#F##. .
      ...#F##. .   ....##F#...
     .  .##F#...  . ...#F##....
    .......##F########F##.. ..
   . .........########......... .
     ..######################..  .
    ...#FFFFFFFFFFFFFFFFFFFF#..
  . ...#FFFFFFFFFFFFFFFFFFFF#... .
  .  ..#FFFFFFFFF  FFFFFFFFF#..
    ...#FFFFFFFF    FFFFFFFF#..
   . ..#FFFFFFFFF  FFFFFFFFF#.. .
  . ...#FFFFFFFFF  FFFFFFFFF#...
    ...#FFFFFFFFFFFFFFFFFFFF#... .
   . ..#FFFFFFFFFFFFFFFFFFFF#....
     ..######################.....
     ..........................
   .  .   . ....    .. .  .  ...`,
  sparkA: `
   :   
   F   
 ..#.. 
:FF#FF:
 ..#.. 
   F   
   :   `,
  sparkB: `
  .  
 .F. 
.FFF.
 .F. 
  .  `,
  sparkC: `
    /    
   /F.   
 ../#/.. 
:F/#F#/F:
 ../#/.. 
   .F/   
    /    `,
} as const;

export type AsciiName = keyof typeof ASCII_ART;

export function Ascii({ name, className = '' }: { name: AsciiName; className?: string }) {
  return (
    <pre aria-hidden="true" className={`ascii ${className}`}>
      {ASCII_ART[name]}
    </pre>
  );
}
