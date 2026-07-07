package service

type CRDService struct {
	manifest []byte
}

func NewCRDService(manifest []byte) *CRDService {
	return &CRDService{
		manifest: manifest,
	}
}

func (s *CRDService) Manifest() []byte {
	return append([]byte(nil), s.manifest...)
}
