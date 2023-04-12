import { assertEquals } from "https://deno.land/std@0.182.0/testing/asserts.ts";

export enum Op {
  NOP = 0x01,
  DROP = 0x1a,
  BLOCK = 0x02,
  LOOP = 0x03,
  LOCAL_GET = 0x20,
  BR = 0x0c,
  RETURN = 0x0f,
  I32_CONST = 0x41,
  I64_CONST = 0x42,
  I64_LE_U = 0x58,
  IF = 0x04,
  I64 = 0x7e,
  ELSE = 0x05,
  I64_SUB = 0x7d,
  CALL = 0x10,
  I64_ADD = 0x7c,
  END = 0x0b,
  I64_EQ = 0x7b,
}

enum LabelKind {
  IF,
  LOOP,
  BLOCK,
}

type Label = {
  kind: LabelKind;
  start?: number; // loopの場合、loop再開時点のpc
  pc: number;
  sp: number;
  arity: number;
};

type Frame = {
  pc: number; // プログラムカウンター
  sp: number; // スタックポインター
  locals: number[]; // 関数の引数やローカル変数が格納される
  label_stack: Label[]; //
  return_arity: number; // 戻り値の数
  insts: number[]; // 命令列
};

type Func = {
  name: string;
  body: number[];
};

export class Runtime {
  stack: number[] = []; // 値スタック
  call_stack: Frame[] = []; // 関数呼び出しフレーム
  funcs: Func[] = []; // 関数テーブル

  constructor(funcs: Func[]) {
    this.funcs = funcs;
  }

  get_else_or_end_address(insts: number[], pc: number): number {
    let depth = 0;
    while (true) {
      pc++;
      if (pc >= insts.length) {
        throw new Error("invalid wasm");
      }
      const inst = insts[pc];
      if (inst === Op.ELSE) { // else
        // 深さが0の場合はifと対応するelseなのでその時点のpcを返す
        if (depth === 0) {
          return pc;
        }
      } else {
        // if の場合は深さを+1
        // この深さはifがネストしている場合に対応するため
        if (inst === Op.IF) {
          depth++;
        } else if (inst === Op.END) {
          // 深さが0の場合はifと対応するendなのでその時点のpcを返す
          if (depth === 0) {
            return pc;
          }
          // end の場合は深さを-1
          depth--;
        }
      }
    }
  }

  // if .. end
  // if .. else .. end
  // loop .. end
  // block .. end
  get_end_address(insts: number[], pc: number): number {
    let depth = 0;
    while (true) {
      pc++;
      if (pc >= insts.length) {
        throw new Error("invalid wasm");
      }
      const inst = insts[pc];
      if (inst === Op.IF || inst === Op.LOOP || inst === Op.BLOCK) {
        depth++;
      } else if (inst === Op.END) {
        if (depth === 0) {
          return pc;
        }
        depth--;
      } else {
        // operand の場合はスキップする
        if (inst === Op.I32_CONST || inst === Op.I64_CONST) {
          pc++;
        }
      }
    }
  }

  stack_unwind(sp: number, arity: number) {
    if (arity > 0) {
      const value = this.stack.pop()!;
      this.stack.splice(sp);
      this.stack.push(value);
    } else {
      this.stack.splice(sp);
    }
  }

  execute() {
    // 命令を実行していく
    while (true) {
      // 呼び出しスタックから現在のフレームを取得
      const frame = this.call_stack[this.call_stack.length - 1];
      // フレームがない場合関数処理が終了したを意味する
      if (!frame) {
        return;
      }
      const insts = frame.insts;

      // 命令がなくなったら終了
      if (frame.pc >= insts.length) {
        break;
      }
      const inst = insts[frame.pc];
      switch (inst) {
        case Op.LOCAL_GET: { // local.get
          frame.pc++;
          const local_index = insts[frame.pc];
          this.stack.push(frame.locals[local_index]);
          break;
        }
        case Op.I32_CONST: {
          frame.pc++;
          const i32_literal = insts[frame.pc];
          this.stack.push(i32_literal);
          break;
        }
        case Op.I64_CONST: { // i64.const
          frame.pc++;
          const i64_literal = insts[frame.pc];
          this.stack.push(i64_literal);
          break;
        }
        case Op.I64_LE_U: { // i64.le_u
          const r = this.stack.pop()!;
          const l = this.stack.pop()!;
          this.stack.push(l <= r ? 1 : 0);
          break;
        }
        case Op.I64_ADD: { // i64.add
          const r = this.stack.pop()!;
          const l = this.stack.pop()!;
          this.stack.push(l + r);
          break;
        }
        case Op.I64_SUB: { // i64.sub
          const r = this.stack.pop()!;
          const l = this.stack.pop()!;
          this.stack.push(l - r);
          break;
        }
        case Op.I64_EQ: {
          const r = this.stack.pop()!;
          const l = this.stack.pop()!;
          this.stack.push(l === r ? 1 : 0);
          break;
        }
        case Op.IF: { // if
          const cond = this.stack.pop()!;
          const sp = this.stack.length;
          frame.pc++;
          let arity = 0;
          const block_type = insts[frame.pc];
          if (block_type === 0x40) {
            // empty
          } else {
            arity = 1;
          }

          // 条件がtrueの場合、if実行後にendにまでpcをすすめるため
          // endのpcを計算してlabelにpushしておく
          // spはスタックを巻き戻するために必要
          // arityは戻り値の数をthis.stackからpopするために必要
          // arityが0以上の場合、this.stack topから値をarity分popして、spまでthis.stackを巻き戻してから
          // popした値をpushする
          if (cond) {
            // if と対応したendのpcを取得
            const next = this.get_end_address(insts, frame.pc);
            frame.label_stack.push({ kind: LabelKind.IF, pc: next, sp, arity });
          } else {
            // falseの場合elseまでpcをすすめる
            frame.pc = this.get_else_or_end_address(insts, frame.pc);
            // else と対応したendのpcを取得してlabelにpush
            const next = this.get_else_or_end_address(insts, frame.pc);
            frame.label_stack.push({ kind: LabelKind.IF, pc: next, sp, arity });
          }
          break;
        }
        case Op.ELSE: { // else
          // else まで来た場合、ifのtrue時のblockの処理が終了したを意味する
          // よって、label を pop して、以下の処理を行う
          //   1. endまでpcをすすめる
          //   2. spまでthis.stackを巻き戻す
          //   3. ifのblockの戻り値をpushする
          const label = frame.label_stack.pop()!;
          const { sp, arity } = label;

          // 次のpc、つまりend時点のpcを設定
          frame.pc = label.pc;

          // spまでthis.stackを巻き戻す
          if (arity > 0) {
            const value = this.stack.pop()!;
            this.stack.splice(sp);
            this.stack.push(value);
          } else {
            this.stack.splice(sp);
          }

          break;
        }
        case Op.CALL: { // call
          // 関数の呼び出し
          // this.call_stackに現在のpcとspをpushしておく
          // 関数の実行が終わったらpopしてpcを戻す
          // spは関数の実行前のスタックの長さを保持しておく
          // 関数の実行後にspまでスタックを巻き戻す
          frame.pc++;
          const func_index = insts[frame.pc];
          const func = this.funcs[func_index];
          const locals = [];
          const value = this.stack.pop();
          if (value !== undefined) {
            locals.push(value);
          }
          this.call_stack.push({
            pc: 0,
            sp: this.stack.length,
            locals,
            label_stack: [],
            return_arity: 1,
            insts: func.body,
          });
          continue;
        }
        case Op.END: { // end
          // endはif、else、loop、block、functionに対応している
          // functionのendの場合、this.call_stackからpopしてpcを戻すため、
          // どの命令のendかを判定する必要がある
          //
          // labelが存在している場合、関数以外の命令のendに対応している
          // labelが存在しない場合、関数のendに対応している
          // 関数のendはthis.call_stackからpopしてpcを戻す必要があるため、分けて処理する必要する
          const label = frame.label_stack.pop();
          if (label) {
            const { sp, arity } = label;
            frame.pc = label.pc;

            // spまでthis.stackを巻き戻す
            this.stack_unwind(sp, arity);
          } else {
            // 関数のendに対応している場合、this.call_stackからpopしてpcを戻す
            const frame = this.call_stack.pop()!;
            const { sp, return_arity } = frame;

            // spまでthis.stackを巻き戻す
            if (return_arity > 0) {
              const value = this.stack.pop()!;
              this.stack.splice(sp);
              this.stack.push(value);
            } else {
              this.stack.splice(sp);
            }
          }
          break;
        }
        case Op.RETURN: { // return
          // returnは関数の終了を意味する
          // よって、call_stackからpopしてpcとstackを戻す
          const frame = this.call_stack.pop()!;
          const { sp, return_arity } = frame;

          // spまでthis.stackを巻き戻す
          this.stack_unwind(sp, return_arity);
          return;
        }
        case Op.LOOP: { // loop
          // loop は pc を書き換えることによって無限ループを実現する
          // 書き換える必要があるかどうかはラベルの種類で判定する
          // br 0 の場合のみループする
          frame.pc++;
          let arity = 0;
          const inst = insts[frame.pc];
          if (inst !== 0x40) {
            arity = 1;
          }

          const start = frame.pc + 1;
          const pc = this.get_end_address(frame.insts, frame.pc);

          const label = {
            kind: LabelKind.LOOP,
            start,
            pc, // ループを抜けた時点のpc
            sp: this.stack.length,
            arity,
          };
          frame.label_stack.push(label);
          break;
        }
        case Op.BLOCK: {
          frame.pc++;
          let arity = 0;
          const inst = insts[frame.pc];
          if (inst !== 0x40) {
            arity = 1;
          }
          const pc = this.get_end_address(frame.insts, frame.pc);
          const label = {
            kind: LabelKind.BLOCK,
            pc,
            sp: this.stack.length,
            arity,
          };
          frame.label_stack.push(label);
          break;
        }
        case Op.BR: { // br
          // ラベルジャンプ
          // br 0 の場合は現在のブロックを抜けるを意味する
          // br 1 は一つ外側のブロックを抜けるを意味する
          // つまりうち側から外に向かってラベルを探していく
          // label_stack は FIFO なので、後ろから辿っていくことになる
          //
          // 注意点: ラベルの種類が loop の場合のみループの先頭に戻る
          frame.pc++;
          const break_depth = insts[frame.pc];
          const label_index = (frame.label_stack.length - 1) - break_depth;
          const label = frame.label_stack[label_index];

          if (label.kind === LabelKind.LOOP) {
            // ループの先頭に戻る
            frame.pc = label.start!;
            this.stack_unwind(label.sp, 0);
          } else {
            // 対象のラベルまでジャンプする
            // つまり、以下の処理を行う
            //   1. label_stack から対象のラベルまでpopする
            //   2. labelのpcにpcを書き換える
            //   3. labelのspまでstackを巻き戻す
            frame.label_stack.splice(label_index);
            frame.pc = label.pc;
            this.stack_unwind(label.sp, label.arity);
          }
          break;
        }
        case Op.NOP: {
          break;
        }
      }
      frame.pc++;
    }
  }

  call(name: string, args: number[]) {
    const func = this.funcs.filter((f) => f.name === name);
    if (func.length == 0) {
      throw new Error(`function ${name} is not found`);
    }
    this.call_stack.push({
      pc: 0,
      sp: this.stack.length,
      locals: args,
      label_stack: [],
      return_arity: 1,
      insts: func[0].body,
    });
    this.execute();
    return this.stack.pop();
  }
}

// ==================== test ============================

Deno.test({
  name: "fib",
  fn: async (t) => {
    const func = {
      name: "fib",
      body: [
        Op.LOCAL_GET, // 0x20 local.get
        Op.I64_CONST, // 0x41 i32.const
        Op.I64_EQ, // 0x46 i32.eq
        Op.IF, // 0x04 if
        Op.I32_CONST, // 0x41 i32.const
        Op.RETURN, // 0x0f return
        Op.END, // 0x0b end
        Op.LOCAL_GET, // 0x20 local.get
        Op.I64_CONST, // 0x41 i32.const
        Op.I64_EQ, // 0x46 i32.eq
        Op.IF, // 0x04 if
        Op.I32_CONST, // 0x41 i32.const
        Op.RETURN, // 0x0f return
        Op.END, // 0x0b end
        Op.LOCAL_GET, // 0x20 local.get
        Op.I64_CONST, // 0x41 i32.const
        Op.I64_SUB, // 0x6b i32.sub
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.LOCAL_GET, // 0x20 local.get
        Op.I32_CONST, // 0x41 i32.const
        Op.I64_SUB, // 0x6b i32.sub
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.I64_ADD, // 0x6a i32.add
        Op.END, // 0x0b end
        Op.END, // 0x0b end
        //Op.LOCAL_GET, // 0 local.get
        //0x00, // 1 local index
        //Op.I64_CONST, // 2 i64.const
        //0x01, // 3 i64 literal
        //Op.I64_LE_U, // 4 i64.le_u
        //Op.IF, // 5 if
        //Op.I64, // 6 i64
        //Op.I64_CONST, // 7 i64.const
        //0x01, // 8 i64 literal
        //Op.ELSE, // 9 else
        //Op.LOCAL_GET, // 10 local.get
        //0x00, // 11 local index
        //Op.I64_CONST, // 12 i64.const
        //0x02, // 13 i64 literal
        //Op.I64_SUB, // 14 i64.sub
        //Op.CALL, // 15 call
        //0x00, // 16 function index
        //Op.LOCAL_GET, // 17 local.get
        //0x00, // 18 local index
        //Op.I64_CONST, // 19 i64.const
        //0x01, // 20 i64 literal
        //Op.I64_SUB, // 21 i64.sub
        //Op.CALL, // 22 call
        //0x00, // 23 function index
        //Op.I64_ADD, // 24 i64.add
        //Op.END, // 25 end
        //Op.END, // 26 end
      ],
    };

    const tests = [
      //{ args: [0], expected: 1 },
      //{ args: [1], expected: 1 },
      //{ args: [2], expected: 2 },
      //{ args: [5], expected: 8 },
      { args: [10], expected: 55 },
      //{ args: [20], expected: 10946 },
    ];

    for (const { args, expected } of tests) {
      await t.step("fib(" + args + ")", () => {
        const runtime = new Runtime([func]);
        const result = runtime.call("fib", args);
        assertEquals(result, expected);
      });
    }
  },
});

Deno.test({
  name: "flow",
  fn: async (t) => {
    const dummy = {
      name: "dummy",
      body: [
        Op.END,
      ],
    };

    const empty = {
      name: "empty",
      body: [
        Op.LOOP, // 0x03 loop
        0x40, // 0x40 void
        Op.END, // 0x0b end
        Op.LOOP, // 0x03 loop
        0x40, // 0x40 void
        Op.END, // 0x0b end
        Op.END, // 0x0b end
      ],
    };

    const singular = {
      name: "singular",
      body: [
        Op.LOOP, // 0x03 loop
        0x40, // 0x40 void
        Op.NOP, // 0x01 nop
        Op.END, // 0x0b end
        Op.LOOP, // 0x03 loop
        0x7f, // 0x7f i32
        Op.I32_CONST, // 0x41 i32.const
        0x07, // i32 literal
        Op.END, // 0x0b end
        Op.END, // 0x0b end
      ],
    };

    const multi = {
      name: "multi",
      body: [
        Op.LOOP, // 0x03 loop
        0x40, // 0x40 void
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.END, // 0x0b end
        Op.LOOP, // 0x03 loop
        0x7f, // 0x7f i32
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.I32_CONST, // 0x41 i32.const
        0x08, // i32 literal
        Op.END, // 0x0b end
        Op.END, // 0x0b end
      ],
    };

    const as_block_value = {
      name: "as_block_value",
      body: [
        Op.BLOCK, // 0x02 block
        0x7f, // 0x7f i32
        Op.NOP, // 0x01 nop
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.I32_CONST, // 0x41 i32.const
        0x02, // i32 literal
        Op.BR, // 0x0c br
        0x00, // break depth
        Op.END, // 0x0b end
        Op.END, // 0x0b end
      ],
    };

    const as_loop_value = {
      name: "as_loop_value",
      body: [
        Op.BLOCK, // 0x02 block
        0x7f, // 0x7f i32
        Op.LOOP, // 0x03 loop
        0x7f, // 0x7f i32
        Op.I32_CONST, // 0x41 i32.const
        0x03, // i32 literal
        Op.BR, // 0x0c br
        0x01, // break depth
        Op.I32_CONST, // 0x41 i32.const
        0x02, // i32 literal
        Op.END, // 0x0b end
        Op.END, // 0x0b end
        Op.END, // 0x0b end
      ],
    };

    const as_loop_mid = {
      name: "as_loop_mid",
      body: [
        Op.LOOP, // 0x03 loop
        0x7f, // 0x7f i32
        Op.CALL, // 0x10 call
        0x00, // function index
        Op.I32_CONST, // 0x41 i32.const
        0x04, // i32 literal
        Op.RETURN, // 0x0f return
        Op.I32_CONST, // 0x41 i32.const
        0x02, // i32 literal
        Op.END, // 0x0b end
        Op.END, // 0x0b end
      ],
    };

    const as_if_then = {
      name: "as_if_then",
      body: [
        Op.I32_CONST, // 0x41 i32.const
        0x01, // i32 literal
        Op.IF, // 0x04 if
        0x7f, // 0x7f i32
        Op.BLOCK, // 0x02 block
        0x7f, // 0x7f i32
        Op.I32_CONST, // 0x41 i32.const
        0x01, // i32 literal
        Op.END, // 0x0b end
        Op.ELSE, // 0x05 else
        Op.I32_CONST, // 0x41 i32.const
        0x02, // i32 literal
        Op.END, // 0x0b end
        Op.END, // 0x0b end
      ],
    };

    const funcs = [
      dummy,
      empty,
      singular,
      multi,
      as_block_value,
      as_loop_value,
      as_loop_mid,
      as_if_then,
    ];

    const tests = [
      { name: "dummy", expected: undefined },
      { name: "empty", expected: undefined },
      { name: "singular", expected: 7 },
      { name: "multi", expected: 8 },
      { name: "as_block_value", expected: 2 },
      { name: "as_loop_value", expected: 3 },
      { name: "as_loop_mid", expected: 4 },
      { name: "as_if_then", expected: 1 },
    ];

    for (const { name, expected } of tests) {
      await t.step(`${name}()`, () => {
        const runtime = new Runtime(funcs);
        const result = runtime.call(name, []);
        assertEquals(result, expected);
      });
    }
  },
});
