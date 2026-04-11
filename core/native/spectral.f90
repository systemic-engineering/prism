! spectral.f90 — Eigensystem computation via LAPACK dsyev.
!
! Thin C-callable wrappers around LAPACK's dsyev for real symmetric
! eigenproblems. Same pattern as prism.f90 — bind(c) interface for Rust FFI.
!
! dsyev computes all eigenvalues and optionally eigenvectors of a real
! symmetric matrix. The Laplacian is always real symmetric, so dsyev is
! the right routine.

module optics_spectral
  use iso_c_binding
  implicit none

  private
  public :: spectral_eigensystem, spectral_eigenvalues, spectral_svd, spectral_singular_values

contains

  ! Full eigensystem: eigenvalues + eigenvectors via dsyev('V', 'U', ...).
  ! Eigenvalues returned in ascending order.
  ! Eigenvectors stored as columns of the output matrix (column-major).
  subroutine spectral_eigensystem(n, matrix, eigenvalues, eigenvectors, info) &
      bind(c, name="spectral_eigensystem")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: matrix(n, n)
    real(c_double), intent(out) :: eigenvalues(n)
    real(c_double), intent(out) :: eigenvectors(n, n)
    integer(c_int), intent(out) :: info

    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork

    ! Copy matrix to eigenvectors (dsyev overwrites in-place)
    eigenvectors = matrix

    ! Query optimal workspace size
    lwork = -1
    call dsyev('V', 'U', n, eigenvectors, n, eigenvalues, work_query, lwork, info)
    lwork = int(work_query(1))
    allocate(work(lwork))

    ! Compute eigensystem
    call dsyev('V', 'U', n, eigenvectors, n, eigenvalues, work, lwork, info)

    deallocate(work)
  end subroutine spectral_eigensystem

  ! Eigenvalues only via dsyev('N', 'U', ...).
  ! Faster — no eigenvector computation.
  subroutine spectral_eigenvalues(n, matrix, eigenvalues, info) &
      bind(c, name="spectral_eigenvalues")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: matrix(n, n)
    real(c_double), intent(out) :: eigenvalues(n)
    integer(c_int), intent(out) :: info

    real(c_double), allocatable :: a(:,:)
    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork

    ! Copy matrix (dsyev overwrites in-place)
    allocate(a(n, n))
    a = matrix

    ! Query optimal workspace size
    lwork = -1
    call dsyev('N', 'U', n, a, n, eigenvalues, work_query, lwork, info)
    lwork = int(work_query(1))
    allocate(work(lwork))

    ! Compute eigenvalues only
    call dsyev('N', 'U', n, a, n, eigenvalues, work, lwork, info)

    deallocate(work)
    deallocate(a)
  end subroutine spectral_eigenvalues

  ! Full SVD: singular values + left/right singular vectors via dgesvd('A','A',...).
  ! Singular values returned in descending order.
  ! U is m×m, VT is n×n (V transposed), stored column-major.
  subroutine spectral_svd(m, n, matrix, singular_values, u, vt, info) &
      bind(c, name="spectral_svd")
    integer(c_int), value, intent(in) :: m, n
    real(c_double), intent(in) :: matrix(m, n)
    real(c_double), intent(out) :: singular_values(min(m, n))
    real(c_double), intent(out) :: u(m, m)
    real(c_double), intent(out) :: vt(n, n)
    integer(c_int), intent(out) :: info

    real(c_double), allocatable :: a(:,:)
    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork, k

    k = min(m, n)

    ! Copy matrix before calling (dgesvd overwrites)
    allocate(a(m, n))
    a = matrix

    ! Query optimal workspace size
    lwork = -1
    call dgesvd('A', 'A', m, n, a, m, singular_values, u, m, vt, n, work_query, lwork, info)
    lwork = int(work_query(1))
    allocate(work(lwork))

    ! Restore copy (workspace query may have modified a)
    a = matrix

    ! Compute full SVD
    call dgesvd('A', 'A', m, n, a, m, singular_values, u, m, vt, n, work, lwork, info)

    deallocate(work)
    deallocate(a)
  end subroutine spectral_svd

  ! Singular values only via dgesvd('N','N',...).
  ! Faster — no U/V computation.
  subroutine spectral_singular_values(m, n, matrix, singular_values, info) &
      bind(c, name="spectral_singular_values")
    integer(c_int), value, intent(in) :: m, n
    real(c_double), intent(in) :: matrix(m, n)
    real(c_double), intent(out) :: singular_values(min(m, n))
    integer(c_int), intent(out) :: info

    real(c_double), allocatable :: a(:,:)
    real(c_double) :: dummy_u(1, 1), dummy_vt(1, 1)
    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork, k

    k = min(m, n)

    ! Copy matrix (dgesvd overwrites in-place)
    allocate(a(m, n))
    a = matrix

    ! Query optimal workspace size
    lwork = -1
    call dgesvd('N', 'N', m, n, a, m, singular_values, dummy_u, 1, dummy_vt, 1, work_query, lwork, info)
    lwork = int(work_query(1))
    allocate(work(lwork))

    ! Restore copy
    a = matrix

    ! Compute singular values only
    call dgesvd('N', 'N', m, n, a, m, singular_values, dummy_u, 1, dummy_vt, 1, work, lwork, info)

    deallocate(work)
    deallocate(a)
  end subroutine spectral_singular_values

end module optics_spectral
