# pip install pdfplumber
import pdfplumber, json
from dataclasses import dataclass
from typing import Dict, List, Tuple

@dataclass
class InstructionSpec:
  name: str
  desc: str = ""
  code: str = ""
  notes: str = ""

NAME = "025510+SourceSerifPro-Bold"
DESC = "0a6d0b+SourceSerifPro-Regular"
CODE = "c52825+RobotoMono-Regular"
NOTES = "025510+SourceSerifPro-Bold"
ITALIC = "a75df5+SourceSerifPro-It"

spec: List[InstructionSpec] = []
with pdfplumber.open("/Users/qazal/Downloads/ref.pdf") as pdf:
  toc = pdf.pages[8].extract_text().splitlines()
  instructions = list(filter(lambda x:len(x.strip()), toc[[i for i,x in enumerate(toc) if "Instructions" in x][0]+1:]))
  start = int(instructions[0].split(".")[-1])+8
  end = int(instructions[-2].split(".")[-1])+8
  for page in pdf.pages[start:end]:
    lines = page.extract_text_lines()
    is_note = False
    for line in lines:
      if (font:=line["chars"][0]["fontname"]) == NAME and len(line["text"].split(" ")) == 2:
        name: str = line["text"].split(" ")[0].lower()
        spec.append(InstructionSpec(name))
        is_note = False
      else:
        if not spec: continue
        txt = line["text"]
        if font == ITALIC: txt = f"_{txt}_"
        if is_note:
          spec[-1].notes = spec[-1].notes+"\n"+txt if spec[-1].notes else txt
        elif font in {DESC, ITALIC}:
          spec[-1].desc = spec[-1].desc+"\n"+txt if spec[-1].desc else txt
        elif font == CODE:
          spec[-1].code = spec[-1].code+"\n"+txt if spec[-1].code else txt
        elif font == NOTES: is_note = True
        else: print(f"WARN: dont know how to parse {font} {line['text']}")

kv: Dict[str, Dict] = {}
for s in spec: kv[s.name] = {"desc":s.desc, "code":s.code, "notes":s.notes}
json.dump(kv, open("/Users/qazal/code/rdna3/ref.json", "w"))
