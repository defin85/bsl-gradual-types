#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import requests
import json
import sys

BASE_URL = "http://localhost:3002"

def test_return_type_quantity():
    """Test 1: Return type for Quantity method"""
    print("=" * 80)
    print("TEST 1: Return type for method Quantity()")
    print("=" * 80)

    # ТЗ = Новый ТаблицаЗначений;  (line 1, 0-based)
    # Кол = ТЗ.Количество();          (line 2, 0-based)
    # Position of variable 'Кол' starts at column 0
    code = "ТЗ = Новый ТаблицаЗначений;\nКол = ТЗ.Количество();"
    payload = {
        "code": code,
        "line": 2,
        "column": 0  # Start of variable name 'Кол'
    }

    try:
        response = requests.post(
            f"{BASE_URL}/api/hover/enhanced",
            json=payload,
            timeout=10
        )
        response.raise_for_status()
        result = response.json()

        print(f"Code: {repr(code)}")
        print(f"Position: line={payload['line']}, column={payload['column']}")
        print("\nRaw Response:")
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # Check if we got proper hover
        has_hover = "hover" in result or "hoverText" in result
        found_in_scope = result.get("foundInScope", False)

        print(f"\nfoundInScope: {found_in_scope}")
        print(f"Has hover: {has_hover}")

        if found_in_scope and "hover" in result:
            hover = result["hover"]
            if "type_hint" in hover:
                type_hint = hover["type_hint"]
                print(f"Type Hint: {type_hint}")
                return "Число" in type_hint

        return False

    except Exception as e:
        print(f"FAIL: Error {e}")
        import traceback
        traceback.print_exc()
        return False

def test_diagnostics_endpoint():
    """Test via diagnostics endpoint which might show inferred types"""
    print("\n" + "=" * 80)
    print("TEST (Diagnostics): Checking type inference through diagnostics")
    print("=" * 80)

    code = "ТЗ = Новый ТаблицаЗначений;\nКол = ТЗ.Количество();"
    payload = {"code": code}

    try:
        response = requests.post(
            f"{BASE_URL}/api/diagnostics",
            json=payload,
            timeout=10
        )
        response.raise_for_status()
        result = response.json()

        print(f"Code: {repr(code)}")
        print("\nDiagnostics Response:")
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # Check if diagnostics show type info
        if "diagnostics" in result:
            diags = result["diagnostics"]
            print(f"\nFound {len(diags)} diagnostics")
            for i, diag in enumerate(diags):
                print(f"  {i}: {diag.get('message', 'N/A')}")

        return True

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_debug_ast():
    """Test AST debugging endpoint"""
    print("\n" + "=" * 80)
    print("TEST (AST Debug): Checking AST structure")
    print("=" * 80)

    code = "ТЗ = Новый ТаблицаЗначений;\nКол = ТЗ.Количество();"
    payload = {"code": code}

    try:
        response = requests.post(
            f"{BASE_URL}/api/debug/ast",
            json=payload,
            timeout=10
        )
        response.raise_for_status()
        result = response.json()

        print(f"Code: {repr(code)}")
        print("\nAST Response (first 500 chars):")
        response_str = json.dumps(result, indent=2, ensure_ascii=False)
        print(response_str[:500])

        return True

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return False

def main():
    print("\nAPI TESTING FOR MILESTONE 3.9\n")

    print("Checking hover endpoint structure...")
    test_return_type_quantity()

    print("\n\nChecking diagnostics endpoint...")
    test_diagnostics_endpoint()

    print("\n\nChecking AST debug endpoint...")
    test_debug_ast()

if __name__ == "__main__":
    main()
