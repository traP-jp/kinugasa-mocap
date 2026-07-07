package k8s

import (
	"fmt"
	"strconv"
	"strings"
)

const dnsLabelMaxLength = 63

func resourceName(prefix, name string) string {
	value := strings.ToLower(prefix + "-" + name)
	value = strings.Map(func(r rune) rune {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') || r == '-' {
			return r
		}
		return '-'
	}, value)
	value = strings.Trim(value, "-")
	if len(value) <= dnsLabelMaxLength {
		return value
	}
	return strings.TrimRight(value[:dnsLabelMaxLength], "-")
}

func serviceDNS(name, namespace string, port int32, protocol string) string {
	return fmt.Sprintf("%s://%s.%s.svc.cluster.local:%d", protocol, name, namespace, port)
}

func int32String(value int32) string {
	return strconv.FormatInt(int64(value), 10)
}
