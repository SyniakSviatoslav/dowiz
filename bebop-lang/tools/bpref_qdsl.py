#!/usr/bin/env python3
"""
B7 qdsl standalone Python parser - mirrors qdsl.bp semantics.

Pure-Python implementation of the qdsl query parser that produces the same
fingerprints as the bebop implementation in selfhost/std/qdsl.bp.

Grammar:
  query := 'q' '{' 'from' ident (join)* ('where' pred)? ('group' 'by' keylist ('agg' agglist)?)? ('order' 'by' expr ('desc')?)? ('limit' int)? '}'

AST nodes allocated in arena with 1-based indexing (offset 0 = null sentinel).
"""

import sys

class QDSLParser:
    """Pure Python qdsl parser mirroring bebop qdsl.bp"""
    
    def __init__(self):
        self.arena = [0] * 4096
        self.apos = 0          # allocation pointer (next free offset)
        self.strbuf = [0] * 512
        self.sbufpos = 0
    
    def _alloc(self, kind, nc, attr):
        """Allocate node at next available offset. Returns offset (0-based, matching bebop .bp)."""
        off = self.apos
        self.apos = off + 3 + nc
        self.arena[off] = kind
        self.arena[off+1] = nc
        self.arena[off+2] = attr
        return off
    
    def _set(self, node, ci, val):
        """Set child at index ci"""
        self.arena[node+3+ci] = val
    
    def _get(self, node, ci):
        """Get child at index ci"""
        return self.arena[node+3+ci]
    
    def _store_str(self, s):
        """Store string in strbuf, return offset (0 on error)"""
        if not s:
            return 0
        off = self.sbufpos
        for c in s:
            self.strbuf[off] = ord(c)
            off += 1
        self.strbuf[off] = 0
        self.sbufpos = off + 1
        return off
    
    def _skip_ws(self, s, p):
        """Skip whitespace characters"""
        n = len(s)
        while p < n and s[p] in ' \t\n\r':
            p += 1
        return p
    
    def _check_keyword(self, s, p, kw):
        """Check if keyword kw is at position p"""
        if p + len(kw) > len(s):
            return False
        return s[p:p+len(kw)] == kw
    
    def _parse_ident(self, s, p):
        """Parse identifier: [a-zA-Z_][a-zA-Z0-9_]*
        Returns (node, new_p) or (0, p) on failure.
        Node kind=10, nc=0, attr=string_offset.
        """
        p = self._skip_ws(s, p)
        n = len(s)
        if p >= n:
            return 0, p
        c = s[p]
        # Check first character: must be letter or underscore
        if not (c.isalpha() or c == '_'):
            return 0, n
        start = p
        p += 1
        while p < n and (s[p].isalnum() or s[p] == '_'):
            p += 1
        name = s[start:p]
        str_off = self._store_str(name)
        if str_off == 0:
            return 0, p
        node = self._alloc(10, 0, str_off)
        return node, p
    
    def _parse_int(self, s, p):
        """Parse integer literal.
        Returns (node, new_p) or (0, p) on failure.
        Node kind=10, nc=0, attr=integer_value.
        """
        p = self._skip_ws(s, p)
        n = len(s)
        if p >= n or not s[p].isdigit():
            return 0, n
        start = p
        v = 0
        while p < n and s[p].isdigit():
            v = v * 10 + (ord(s[p]) - 48)
            p += 1
        node = self._alloc(10, 0, v)
        return node, p
    
    def _parse_cmp(self, s, p):
        """Parse comparison: ident op ident
        Returns (node, new_p) or (0, p) on failure.
        Node kind=8, nc=2, attr=op_code.
        op_codes: 1==, 2!=, 3<=, 4>=, 5<, 6>
        """
        p = self._skip_ws(s, p)
        n = len(s)
        left, p = self._parse_ident(s, p)
        if left == 0:
            return 0, p
        p = self._skip_ws(s, p)
        if p >= n:
            return 0, p
        # Check operator
        if p+1 <= n and s[p:p+2] == '==':
            op = 1
            p += 2
        elif p+1 <= n and s[p:p+2] == '!=':
            op = 2
            p += 2
        elif p+1 <= n and s[p:p+2] == '<=':
            op = 3
            p += 2
        elif p+1 <= n and s[p:p+2] == '>=':
            op = 4
            p += 2
        elif s[p] == '<':
            op = 5
            p += 1
        elif s[p] == '>':
            op = 6
            p += 1
        else:
            return 0, p
        p = self._skip_ws(s, p)
        right, p = self._parse_ident(s, p)
        if right == 0:
            return 0, p
        node = self._alloc(8, 2, op)
        self._set(node, 0, left)
        self._set(node, 1, right)
        return node, p
    
    def _parse_pred(self, s, p):
        """Parse predicate: cmp ('and' cmp)*
        Returns (node, new_p) or (0, p) on failure.
        """
        p = self._skip_ws(s, p)
        root, p = self._parse_cmp(s, p)
        if root == 0:
            return 0, p
        p = self._skip_ws(s, p)
        while p+3 <= len(s) and s[p:p+3] == 'and':
            p += 3
            p = self._skip_ws(s, p)
            right, p = self._parse_cmp(s, p)
            if right == 0:
                break
            node = self._alloc(9, 2, 0)  # kind=AND
            self._set(node, 0, root)
            self._set(node, 1, right)
            root = node
            p = self._skip_ws(s, p)
        return root, p
    
    def _parse_agg(self, s, p):
        """Parse aggregate: (sum|count|min|max|avg) '(' ident ')'
        Returns (node, new_p) or (0, p) on failure.
        """
        p = self._skip_ws(s, p)
        n = len(s)
        if p+3 <= n and s[p:p+3] == 'sum':
            ak = 11
            p += 3
        elif p+5 <= n and s[p:p+5] == 'count':
            ak = 12
            p += 5
        elif p+3 <= n and s[p:p+3] == 'min':
            ak = 13
            p += 3
        elif p+3 <= n and s[p:p+3] == 'max':
            ak = 14
            p += 3
        elif p+3 <= n and s[p:p+3] == 'avg':
            ak = 15
            p += 3
        else:
            return 0, p
        p = self._skip_ws(s, p)
        if p >= n or s[p] != '(':
            return 0, p
        p += 1
        p = self._skip_ws(s, p)
        col, p = self._parse_ident(s, p)
        if col == 0:
            return 0, p
        p = self._skip_ws(s, p)
        if p >= n or s[p] != ')':
            return 0, p
        p += 1
        node = self._alloc(ak, 1, 0)
        self._set(node, 0, col)
        return node, p
    
    def parse(self, query):
        """Parse a qdsl query string.
        Returns root node offset (>0) or 0 on error.
        """
        s = query
        n = len(s)
        
        # Skip leading whitespace
        p = self._skip_ws(s, 0)
        
        # 'q' keyword
        if p >= n or s[p] != 'q':
            return 0
        p += 1
        
        # '{' 
        p = self._skip_ws(s, p)
        if p >= n or s[p] != '{':
            return 0
        p += 1
        
        # 'from' keyword
        p = self._skip_ws(s, p)
        if not self._check_keyword(s, p, 'from'):
            return 0
        p += 4
        
        # 'from' table name (ident)
        from_node, p = self._parse_ident(s, p)
        if from_node == 0:
            return 0
        
        # joins: (join table on key (= key)?)*
        joins = []
        p = self._skip_ws(s, p)
        while p+4 <= n and self._check_keyword(s, p, 'join'):
            p += 4
            p = self._skip_ws(s, p)
            jtable, p = self._parse_ident(s, p)
            if jtable == 0:
                return 0
            p = self._skip_ws(s, p)
            if not self._check_keyword(s, p, 'on'):
                return 0
            p += 2
            p = self._skip_ws(s, p)
            jkey, p = self._parse_ident(s, p)
            if jkey == 0:
                return 0
            jrkey = 0
            p = self._skip_ws(s, p)
            if p < n and s[p] == '=':
                p += 1
                p = self._skip_ws(s, p)
                jrkey, p = self._parse_ident(s, p)
                if jrkey == 0:
                    return 0
            nc = 3 if jrkey else 2
            jnode = self._alloc(3, nc, 0)  # kind=JOIN
            self._set(jnode, 0, jtable)
            self._set(jnode, 1, jkey)
            if jrkey:
                self._set(jnode, 2, jrk)
            joins.append(jnode)
            p = self._skip_ws(s, p)
        
        # 'where' clause (optional)
        where_node = 0
        p = self._skip_ws(s, p)
        if self._check_keyword(s, p, 'where'):
            p += 5
            p = self._skip_ws(s, p)
            where_node, p = self._parse_pred(s, p)
            if where_node == 0:
                return 0
        
        # 'group' 'by' clause (optional)
        group_node = 0
        p = self._skip_ws(s, p)
        if self._check_keyword(s, p, 'group'):
            p += 5
            p = self._skip_ws(s, p)
            if not self._check_keyword(s, p, 'by'):
                return 0
            p += 2
            p = self._skip_ws(s, p)
            gkey, p = self._parse_ident(s, p)
            if gkey == 0:
                return 0
            agg_node = 0
            p = self._skip_ws(s, p)
            if self._check_keyword(s, p, 'agg'):
                p += 3
                p = self._skip_ws(s, p)
                agg_node, p = self._parse_agg(s, p)
                if agg_node == 0:
                    return 0
            nc = 1 + (1 if agg_node else 0)
            group_node = self._alloc(5, nc, 0)  # kind=GROUPBY
            self._set(group_node, 0, gkey)
            if agg_node:
                self._set(group_node, 1, agg_node)
        
        # 'order' 'by' clause (optional)
        order_node = 0
        p = self._skip_ws(s, p)
        if self._check_keyword(s, p, 'order'):
            p += 5
            p = self._skip_ws(s, p)
            if not self._check_keyword(s, p, 'by'):
                return 0
            p += 2
            p = self._skip_ws(s, p)
            oexpr, p = self._parse_ident(s, p)
            if oexpr == 0:
                return 0
            desc = 0
            p = self._skip_ws(s, p)
            if self._check_keyword(s, p, 'desc'):
                desc = 1
                p += 4
            order_node = self._alloc(6, 1, desc)  # kind=ORDERBY
            self._set(order_node, 0, oexpr)
        
        # 'limit' clause (optional)
        limit_node = 0
        p = self._skip_ws(s, p)
        if self._check_keyword(s, p, 'limit'):
            p += 5
            p = self._skip_ws(s, p)
            lint, p = self._parse_int(s, p)
            if lint == 0:
                return 0
            limit_val = self.arena[lint+2]  # int value stored in attr field
            limit_node = self._alloc(7, 0, limit_val)  # kind=LIMIT
        
        # '}' closing brace
        p = self._skip_ws(s, p)
        if p >= n or s[p] != '}':
            return 0
        
        # Build root QUERY node
        njoins = len(joins)
        nc = 1 + njoins + (1 if where_node else 0) + (1 if group_node else 0) + (1 if order_node else 0) + (1 if limit_node else 0)
        root = self._alloc(1, nc, 0)  # kind=QUERY
        ci = 0
        self._set(root, ci, from_node)
        ci += 1
        for jn in joins:
            self._set(root, ci, jn)
            ci += 1
        if where_node:
            self._set(root, ci, where_node)
            ci += 1
        if group_node:
            self._set(root, ci, group_node)
            ci += 1
        if order_node:
            self._set(root, ci, order_node)
            ci += 1
        if limit_node:
            self._set(root, ci, limit_node)
            ci += 1
        return root
    
    def fingerprint(self, node):
        """Compute deterministic fingerprint of AST node."""
        if node == 0:
            return 0
        kind = self.arena[node]
        nc = self.arena[node+1]
        attr = self.arena[node+2]
        h = (kind * 1000 + attr) & 0xFFFFFFFFFFFFFFFF
        for ci in range(nc):
            child = self.arena[node+3+ci]
            if child:
                h = (h * 31 + self.fingerprint(child)) & 0xFFFFFFFFFFFFFFFF
        return h


def qdsl_parse(query_str):
    """Parse a qdsl query string and return fingerprint, or 0 on error."""
    parser = QDSLParser()
    root = parser.parse(query_str)
    if root == 0:
        return 0
    return parser.fingerprint(root)


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print('Usage: bpref_qdsl.py <query>', file=sys.stderr)
        sys.exit(2)
    result = qdsl_parse(sys.argv[1])
    if result == 0:
        print('parse error', file=sys.stderr)
        sys.exit(89)
    print(result)
