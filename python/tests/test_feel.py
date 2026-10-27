import datetime
import unittest
from decimal import Decimal

from tok import feel


def ev(text, **ctx):
    return feel.eval_text(text, ctx)


class FeelExpressionTest(unittest.TestCase):
    def test_arithmetic_is_decimal(self):
        self.assertEqual(ev("cena + 12.5", cena=89.9), Decimal("102.4"))
        self.assertEqual(ev("0.1 + 0.2"), Decimal("0.3"))
        self.assertEqual(ev("-(2 * 3) / 4"), Decimal("-1.5"))
        self.assertIsNone(ev("1 / 0"))

    def test_strings_and_functions(self):
        self.assertEqual(ev('"Zamówienie: " + tytul', tytul="BPMN"), "Zamówienie: BPMN")
        self.assertEqual(ev('string length("abc")'), Decimal(3))
        self.assertEqual(ev('upper case("ab")'), "AB")
        self.assertTrue(ev('starts with(kod, "PL")', kod="PL-01"))
        self.assertEqual(ev('substring("biblioteka", 1, 5)'), "bibli")
        self.assertEqual(ev('substring("biblioteka", -4)'), "teka")
        self.assertEqual(ev("round half up(2.345, 2)"), Decimal("2.35"))
        self.assertEqual(ev("max(1, 7, 3)"), Decimal(7))
        self.assertEqual(ev("sum([1, 2, 3])"), Decimal(6))
        self.assertEqual(ev("count(x)", x=[1, 2]), Decimal(2))
        self.assertEqual(ev("string(12.50)"), "12.5")
        self.assertEqual(ev('number("12.5") * 2'), Decimal("25.0"))
        self.assertEqual(ev('date("2027-01-15")'), datetime.date(2027, 1, 15))

    def test_comparisons_and_logic(self):
        self.assertIs(ev('tryb = "kierownik"', tryb="kierownik"), True)
        self.assertIs(ev("zgoda = true", zgoda=True), True)
        self.assertIs(ev('cena > 150 and rodzaj = "drukowana"', cena=200, rodzaj="drukowana"), True)
        self.assertIs(ev("not(a)", a=False), True)
        self.assertIs(ev("x between 1 and 10", x=10), True)
        self.assertIs(ev("x in [1..10)", x=10), False)
        self.assertIs(ev("x in [1, 2, 3]", x=2), True)
        self.assertEqual(ev('if cena > 150 then "drogo" else "tanio"', cena=100), "tanio")
        self.assertIs(ev("osoba.wiek >= 18", osoba={"wiek": 20}), True)

    def test_null_semantics(self):
        self.assertIsNone(ev("x > 1"))
        self.assertIs(ev("x = null"), True)
        self.assertIs(ev("1 = null"), False)
        self.assertIsNone(ev('1 = "1"'))
        self.assertIs(ev("x > 1 or true"), True)
        self.assertIs(ev("x > 1 and false"), False)
        self.assertIsNone(ev("x > 1 and true"))

    def test_checks(self):
        self.assertIsNone(feel.check_expr('tryb = "kierownik"'))
        self.assertEqual(feel.check_expr("cena >")[0], "F002")
        self.assertEqual(feel.check_expr("{a: 1}")[0], "F001")
        self.assertEqual(feel.check_expr("for x in y return x")[0], "F001")
        self.assertEqual(feel.check_expr("sqrt(4)")[0], "F001")
        self.assertEqual(feel.check_expr("2 ** 3")[0], "F001")
        self.assertEqual(feel.check_expr("")[0], "F002")


class FeelUnaryTest(unittest.TestCase):
    def t(self, cell, value, **ctx):
        return feel.test(feel.parse_unary_tests(cell), value, ctx)

    def test_unary_tests(self):
        self.assertTrue(self.t("-", None))
        self.assertTrue(self.t("<= 150", 150))
        self.assertFalse(self.t("<= 150", Decimal("150.01")))
        self.assertTrue(self.t('"e-book"', "e-book"))
        self.assertTrue(self.t('"a", "b"', "b"))
        self.assertTrue(self.t("[18..65]", 18))
        self.assertFalse(self.t("]18..65]", 18))
        self.assertTrue(self.t("(1..5)", 3))
        self.assertTrue(self.t("[1..5[", 4))
        self.assertTrue(self.t('not("student")', "pracownik"))
        self.assertFalse(self.t('not("student", "doktorant")', "student"))
        self.assertTrue(self.t("< limit", 3, limit=5))
        self.assertTrue(self.t("true", True))
        self.assertTrue(self.t("-5", -5))
        self.assertFalse(self.t("> 10", None))

    def test_unary_checks(self):
        self.assertIsNone(feel.check_unary("[18..65]"))
        self.assertEqual(feel.check_unary("")[0], "F002")
        self.assertEqual(feel.check_unary("< ")[0], "F002")


if __name__ == "__main__":
    unittest.main()
