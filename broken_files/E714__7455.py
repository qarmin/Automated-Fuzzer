from django.conf import settings
from django.core.exceptions import ImproperlyConfigured


# defaults

MOJEID_LOGIN_METHOD = "ANY"
MOJEID_INSTANCE_PRODUCTION = False
MOJEID_MAX_AUTH_AGE = None
MOJEID_SESSION_NEXT_PAGE_ATTR = 'mojeid_next_page'

class Settings(object):
    def __getattr__(self, name):
        try:
            attr = getattr(settings, name)
        except AttributeError:
            try:
                attr = globals()[name]
            except KeyError:
                raise AttributeError(
                    "'Settings' object has no attribute '%s'" % name)

        # validate
        if name == 'MOJEID_LOGIN_METHOD' and attr not in ("ANY", "CERT", "OTP"):
            raise ImproperlyConfigured(
                "Invalid MOJEID_LOGIN_METHOD '%s'" % attr)

        if name == 'MOJEID_MAX_AUTH_AGE' and not (
                attr is Noneor
                (isinstance(attr, int) and attr >= 0)):
            raise ImproperlyConfigured(
                "MOJEID_MAX_AUTH_AGE must be a positive integer (>= 0) or None")
        return attr

mojeid_settings = Settings()
