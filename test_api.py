#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import requests
import json
import sys
import os

# Fix encoding for Windows
os.environ["PYTHONIOENCODING"] = "utf-8"

BASE_URL = "http://localhost:3002"

def test_return_type_quantity():
    """Test 1: Return type for Quantity method"""
    print("=" * 80)
    print("TEST 1: Return type for method Kolichestvo()")
    print("=" * 80)

    code = "ТЗ = Новый ТаблицаЗначений;\nКол = ТЗ.Количество();"
    payload = {
        "code": code,
        "line": 2,
        "column": 6
    }

    try:
        response = requests.post(
            f"{BASE_URL}/api/hover/enhanced",
            json=payload,
            timeout=10
        )
        response.raise_for_status()
        result = response.json()

        print(f"Code:\n{code}\n")
        print(f"Position: line={payload['line']}, column={payload['column']}\n")
        print("Response:")
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # Check result
        if "hover" in result:
            hover = result["hover"]
            if "type_hint" in hover:
                type_hint = hover["type_hint"]
                print(f"\nType Hint: {type_hint}")
                if "Number" in type_hint or "Число" in type_hint:
                    print("PASS: Correct type 'Number' for Quantity()")
                    return True
                else:
                    print(f"FAIL: Expected 'Number', got '{type_hint}'")
                    return False

        print("FAIL: type_hint not found in response")
        return False

    except Exception as e:
        print(f"FAIL: Error {e}")
        import traceback
        traceback.print_exc()
        return False

def test_global_function_typznch():
    """Test 2: Return type for global function TypeOf()"""
    print("\n" + "=" * 80)
    print("TEST 2: Return type for global function TypeOf()")
    print("=" * 80)

    code = "ТЗ = Новый ТаблицаЗначений;\nТип = ТипЗнч(ТЗ);"
    payload = {
        "code": code,
        "line": 2,
        "column": 5
    }

    try:
        response = requests.post(
            f"{BASE_URL}/api/hover/enhanced",
            json=payload,
            timeout=10
        )
        response.raise_for_status()
        result = response.json()

        print(f"Code:\n{code}\n")
        print(f"Position: line={payload['line']}, column={payload['column']}\n")
        print("Response:")
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # Check result
        if "hover" in result:
            hover = result["hover"]
            if "type_hint" in hover:
                type_hint = hover["type_hint"]
                print(f"\nType Hint: {type_hint}")
                if "Type" in type_hint or "Тип" in type_hint:
                    print("PASS: Correct type 'Type' for TypeOf()")
                    return True
                else:
                    print(f"FAIL: Expected 'Type', got '{type_hint}'")
                    return False

        print("FAIL: type_hint not found in response")
        return False

    except Exception as e:
        print(f"FAIL: Error {e}")
        import traceback
        traceback.print_exc()
        return False

def test_void_method():
    """Test 3: Void method (procedure)"""
    print("\n" + "=" * 80)
    print("TEST 3: Void method - Message()")
    print("=" * 80)

    code = "Сообщить(\"Привет\");"
    payload = {
        "code": code,
        "line": 1,
        "column": 0
    }

    try:
        response = requests.post(
            f"{BASE_URL}/api/hover/enhanced",
            json=payload,
            timeout=10
        )
        response.raise_for_status()
        result = response.json()

        print(f"Code:\n{code}\n")
        print(f"Position: line={payload['line']}, column={payload['column']}\n")
        print("Response:")
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # For void methods might be Undefined or empty
        if "hover" in result:
            hover = result["hover"]
            print("PASS: Hover information received for void method")
            if "type_hint" in hover:
                type_hint = hover["type_hint"]
                print(f"  Type Hint: {type_hint}")
            return True

        print("PASS: Response received (void method processed)")
        return True

    except Exception as e:
        print(f"FAIL: Error {e}")
        import traceback
        traceback.print_exc()
        return False

def main():
    print("\nFUNCTIONAL TESTING OF MILESTONE 3.9\n")

    results = []
    results.append(("Test 1: Quantity()", test_return_type_quantity()))
    results.append(("Test 2: TypeOf()", test_global_function_typznch()))
    results.append(("Test 3: Void method", test_void_method()))

    print("\n" + "=" * 80)
    print("RESULTS")
    print("=" * 80)
    for name, passed in results:
        status = "PASS" if passed else "FAIL"
        print(f"{status}: {name}")

    passed_count = sum(1 for _, p in results if p)
    total_count = len(results)
    print(f"\nTotal: {passed_count}/{total_count} passed")

    return 0 if passed_count == total_count else 1

if __name__ == "__main__":
    sys.exit(main())
