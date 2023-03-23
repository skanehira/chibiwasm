type BLOCK_KIND = "block" | "if" | "loop";
type RESULT_TYPE = "i32" | "i64" | "f32" | "f64";
type Label = {
  result_type?: RESULT_TYPE[];
  block_kind: BLOCK_KIND;
  pc: number;
  sp: number;
};

let pc = 0;
const label_stack: Label[] = [];
const value_stack: number[] = [];
const local_stack: number[] = Array(1);
const instructions = [
  0x41, // i32.const
  0x00, // i32 literal
  0x21, // local.set
  0x00, // local index
  0x02, // block
  0x7f, // i32
  0x03, // loop
  0x7f, // i32
  0x20, // local.get
  0x00, // local index
  0x41, // i32.const
  0x01, // i32 literal
  0x6a, // i32.add
  0x21, // local.set
  0x00, // local index
  0x20, // local.get
  0x00, // local index
  0x41, // i32.const
  0x05, // i32 literal
  0x46, // i32.eq
  0x04, // if
  0x40, // void
  0x20, // local.get
  0x00, // local index
  0x0c, // br
  0x02, // break depth
  0x0b, // end
  0x20, // local.get
  0x00, // local index
  0x0b, // end
  0x0b, // end
];

const result_type_map = new Map<number, RESULT_TYPE>([
  [0x7f, "i32"],
]);

function add_label(
  pc: number,
  sp: number,
  block_kind: BLOCK_KIND,
  result_type?: RESULT_TYPE[],
) {
  label_stack.push({
    result_type: result_type,
    block_kind: block_kind,
    pc: pc + 1,
    sp: sp,
  });
}

function get_blocktype() {
  const result_type = result_type_map.get(instructions[pc]);
  // TODO: 複数のresult typeを取得
  return result_type ? [result_type] : undefined;
}

function skip_until_else_or_end() {
  let depth = 1;
  while (depth !== 0) {
    pc++;
    const inst = instructions[pc];
    switch (inst) {
      case 0x04: // if
      case 0x7f: // block
      case 0x03: // loop
        depth++;
        break;
      case 0x0b: // end
      case 0x05: // else
        depth--;
        pc--;
        break;
    }
  }
}

function execute() {
  while (instructions.length > pc) {
    const inst = instructions[pc];
    switch (inst) {
      case 0x41: // i32.const
        {
          pc++;
          const operand = instructions[pc];
          value_stack.push(operand);
        }
        break;
      case 0x6a: // i32.add
        {
          const right = value_stack.pop()!;
          const left = value_stack.pop()!;
          value_stack.push(left + right);
        }
        break;
      case 0x46:
        {
          const right = value_stack.pop()!;
          const left = value_stack.pop()!;
          value_stack.push((left === right) ? 1 : 0);
        }
        break;
      case 0x20: // local.get
        {
          pc++;
          const index = instructions[pc];
          const value = local_stack[index];
          value_stack.push(value);
        }
        break;
      case 0x21: // local.set
        {
          pc++;
          const index = instructions[pc];
          const value = value_stack.pop();
          local_stack[index] = value!;
        }
        break;
      case 0x02: // block
        {
          pc++;
          const result_types = get_blocktype();
          add_label(
            pc - 1,
            value_stack.length,
            "block",
            result_types,
          );
        }
        break;
      case 0x03: // loop
        {
          pc++;
          const result_types = get_blocktype();
          add_label(pc - 1, value_stack.length, "loop", result_types);
        }
        break;
      case 0x04: // if
        {
          pc++;
          const result_types = get_blocktype();
          add_label(pc - 1, value_stack.length, "if", result_types);
          const should_enter = value_stack.pop() === 1;

          // 条件がfalseの場合 else または end まで skip
          if (!should_enter) {
            skip_until_else_or_end();
          }
        }
        break;
      case 0x05: // else
        break;
      case 0x0c: // br
        {
          pc++;
          const break_depth = instructions[pc];
          let depth = label_stack.length;

          const label = label_stack[depth - break_depth - 1];
          switch (label.block_kind) {
            case "block":
            case "if":
              {
                // skip until end
                while (depth !== 0) {
                  pc++;
                  const inst = instructions[pc];

                  if (inst === 0x0b) { // End
                    label_stack.pop();
                    depth--;
                  }
                }
              }
              break;
            case "loop":
              // loop ラベルまでスタックを縮める
              label_stack.splice(depth - break_depth);
              pc = label.pc;
              break;
          }

          if (label.result_type) {
            const value = value_stack.pop()!;
            while (value_stack.length > label.sp) {
              value_stack.pop();
            }
            if (value) {
              value_stack.push(value);
            }
          } else {
            while (value_stack.length > label.sp) {
              value_stack.pop();
            }
          }
        }
        break;
      case 0x0b: // end
        label_stack.pop();
        break;
    }

    pc++;
  }
}

execute();
console.log(value_stack);
