// Zinc, the bare metal stack for rust.
// Copyright 2014 Ben Gamari <bgamari@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![crate_name="macro_ioreg"]
#![crate_type="dylib"]

#![feature(plugin_registrar, quote)]

extern crate rustc;
extern crate syntax;
extern crate serialize;
extern crate ioreg;

use std::io;
use std::io::fs;
use serialize::json;

use rustc::plugin::Registry;
use syntax::ast;
use syntax::parse::token;
use syntax::ptr::P;
use syntax::codemap::Span;
use syntax::ext::base::{ExtCtxt, MacResult};
use syntax::util::small_vector::SmallVector;

use ioreg::parser::Parser;
use ioreg::builder::Builder;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
  reg.register_macro("ioregs", macro_ioregs);
}

pub fn get_reg_dump_root(cx: &ExtCtxt) -> Option<Path> {
  for mi in cx.cfg.iter() {
    match mi.node {
      ast::MetaNameValue(ref name, ref value) if name.equiv(&"reg_dump") => {
        match value.node {
          ast::LitStr(ref s, _) => return Some(Path::new(s.get())),
          _ => fail!("Invalid `reg_dump` cfg value"),
        }
      },
      _ => {}
    }
  }
  return None;
}

pub fn macro_ioregs(cx: &mut ExtCtxt, _: Span, tts: &[ast::TokenTree])
                    -> Box<MacResult+'static> {
  match Parser::new(cx, tts).parse_ioregs() {
    Some(group) => {
      match get_reg_dump_root(cx) {
        None => {},
        Some(mut path) => {
          for id in cx.mod_path.iter() {
            path.push(token::get_ident(*id).get().into_string());
          }
          fs::mkdir_recursive(&path, io::USER_RWX).unwrap();
          path.push(group.name.node+".json");
          let mut file = fs::File::create(&path).unwrap();
          file.write_str(json::encode(&*group).as_slice());
        }
      }

      let mut builder = Builder::new();
      let items = builder.emit_items(cx, group);
      MacItems::new(items)
    },
    None => {
      fail!();
    }
  }
}

pub struct MacItems {
  items: Vec<P<ast::Item>>
}

impl MacItems {
  pub fn new(items: Vec<P<ast::Item>>) -> Box<MacResult+'static> {
    box MacItems { items: items } as Box<MacResult>
  }
}

impl MacResult for MacItems {
  fn make_items(self: Box<MacItems>) -> Option<SmallVector<P<ast::Item>>> {
    Some(SmallVector::many(self.items.clone()))
  }
}
