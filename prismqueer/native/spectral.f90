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
  public :: spectral_eigensystem, spectral_eigenvalues, spectral_svd, spectral_singular_values, spectral_phase_lock
  public :: spectral_allocate_probe

contains

  ! Full eigensystem: eigenvalues + eigenvectors via dsyev('V', 'U', ...).
  ! Eigenvalues returned in ascending order.
  ! Eigenvectors stored as columns of the output matrix (column-major).
  subroutine spectral_eigensystem(n, matrix, eigenvalues, eigenvectors, info, stat) &
      bind(c, name="spectral_eigensystem")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: matrix(n, n)
    real(c_double), intent(out) :: eigenvalues(n)
    real(c_double), intent(out) :: eigenvectors(n, n)
    integer(c_int), intent(out) :: info
    integer(c_int), intent(out) :: stat

    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork

    stat = 0

    ! Copy matrix to eigenvectors (dsyev overwrites in-place)
    eigenvectors = matrix

    ! Query optimal workspace size
    lwork = -1
    call dsyev('V', 'U', n, eigenvectors, n, eigenvalues, work_query, lwork, info)
    if (info /= 0) return
    call workspace_size(work_query(1), lwork, stat)
    if (stat /= 0) return
    allocate(work(lwork), stat=stat)
    if (stat /= 0) return

    ! Compute eigensystem
    call dsyev('V', 'U', n, eigenvectors, n, eigenvalues, work, lwork, info)

    deallocate(work)
  end subroutine spectral_eigensystem

  ! Eigenvalues only via dsyev('N', 'U', ...).
  ! Faster — no eigenvector computation.
  subroutine spectral_eigenvalues(n, matrix, eigenvalues, info, stat) &
      bind(c, name="spectral_eigenvalues")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: matrix(n, n)
    real(c_double), intent(out) :: eigenvalues(n)
    integer(c_int), intent(out) :: info
    integer(c_int), intent(out) :: stat

    real(c_double), allocatable :: a(:,:)
    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork

    stat = 0

    ! Copy matrix (dsyev overwrites in-place)
    allocate(a(n, n), stat=stat)
    if (stat /= 0) return
    a = matrix

    ! Query optimal workspace size
    lwork = -1
    call dsyev('N', 'U', n, a, n, eigenvalues, work_query, lwork, info)
    if (info /= 0) return
    call workspace_size(work_query(1), lwork, stat)
    if (stat /= 0) return
    allocate(work(lwork), stat=stat)
    if (stat /= 0) return

    ! Compute eigenvalues only
    call dsyev('N', 'U', n, a, n, eigenvalues, work, lwork, info)

    deallocate(work)
    deallocate(a)
  end subroutine spectral_eigenvalues

  ! Full SVD: singular values + left/right singular vectors via dgesvd('A','A',...).
  ! Singular values returned in descending order.
  ! U is m×m, VT is n×n (V transposed), stored column-major.
  subroutine spectral_svd(m, n, matrix, singular_values, u, vt, info, stat) &
      bind(c, name="spectral_svd")
    integer(c_int), value, intent(in) :: m, n
    real(c_double), intent(in) :: matrix(m, n)
    real(c_double), intent(out) :: singular_values(min(m, n))
    real(c_double), intent(out) :: u(m, m)
    real(c_double), intent(out) :: vt(n, n)
    integer(c_int), intent(out) :: info
    integer(c_int), intent(out) :: stat

    real(c_double), allocatable :: a(:,:)
    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork, k

    k = min(m, n)
    stat = 0

    ! Copy matrix before calling (dgesvd overwrites)
    allocate(a(m, n), stat=stat)
    if (stat /= 0) return
    a = matrix

    ! Query optimal workspace size
    lwork = -1
    call dgesvd('A', 'A', m, n, a, m, singular_values, u, m, vt, n, work_query, lwork, info)
    if (info /= 0) return
    call workspace_size(work_query(1), lwork, stat)
    if (stat /= 0) return
    allocate(work(lwork), stat=stat)
    if (stat /= 0) return

    ! Restore copy (workspace query may have modified a)
    a = matrix

    ! Compute full SVD
    call dgesvd('A', 'A', m, n, a, m, singular_values, u, m, vt, n, work, lwork, info)

    deallocate(work)
    deallocate(a)
  end subroutine spectral_svd

  ! Singular values only via dgesvd('N','N',...).
  ! Faster — no U/V computation.
  subroutine spectral_singular_values(m, n, matrix, singular_values, info, stat) &
      bind(c, name="spectral_singular_values")
    integer(c_int), value, intent(in) :: m, n
    real(c_double), intent(in) :: matrix(m, n)
    real(c_double), intent(out) :: singular_values(min(m, n))
    integer(c_int), intent(out) :: info
    integer(c_int), intent(out) :: stat

    real(c_double), allocatable :: a(:,:)
    real(c_double) :: dummy_u(1, 1), dummy_vt(1, 1)
    real(c_double) :: work_query(1)
    real(c_double), allocatable :: work(:)
    integer :: lwork, k

    k = min(m, n)
    stat = 0

    ! Copy matrix (dgesvd overwrites in-place)
    allocate(a(m, n), stat=stat)
    if (stat /= 0) return
    a = matrix

    ! Query optimal workspace size
    lwork = -1
    call dgesvd('N', 'N', m, n, a, m, singular_values, dummy_u, 1, dummy_vt, 1, work_query, lwork, info)
    if (info /= 0) return
    call workspace_size(work_query(1), lwork, stat)
    if (stat /= 0) return
    allocate(work(lwork), stat=stat)
    if (stat /= 0) return

    ! Restore copy
    a = matrix

    ! Compute singular values only
    call dgesvd('N', 'N', m, n, a, m, singular_values, dummy_u, 1, dummy_vt, 1, work, lwork, info)

    deallocate(work)
    deallocate(a)
  end subroutine spectral_singular_values

  ! Kuramoto phase-lock integration for N ≥ 2 coupled oscillators.
  ! Model: dθ_i/dt = ω_i + (K/N) Σ_j sin(θ_j - θ_i)
  ! Explicit Euler integration for `steps` timesteps of `dt`.
  ! Returns final phases + order parameter r = |(1/N) Σ_j e^(iθ_j)| ∈ [0,1].
  ! r=1 = full synchronization; r=0 = incoherent (evenly distributed phases).
  !
  ! Reed 2026-07-20 per Alex "Not DEFERRED. DONE!" FLANG-floor directive.
  ! Numerical work lives in Fortran (below the knife); Rust delegates the
  ! entire Kuramoto integration through this bind(c) surface.
  subroutine spectral_phase_lock(n, phases_in, omegas, k, steps, dt, phases_out, order_r, info, stat) &
      bind(c, name="spectral_phase_lock")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: phases_in(n)
    real(c_double), intent(in) :: omegas(n)
    real(c_double), value, intent(in) :: k
    integer(c_int), value, intent(in) :: steps
    real(c_double), value, intent(in) :: dt
    real(c_double), intent(out) :: phases_out(n)
    real(c_double), intent(out) :: order_r
    integer(c_int), intent(out) :: info
    integer(c_int), intent(out) :: stat

    real(c_double), allocatable :: theta(:), dtheta(:)
    real(c_double) :: cos_sum, sin_sum, k_over_n
    integer :: i, j, s

    info = 0
    stat = 0
    if (n < 1) then
      ! N is argument 1, refused the way LAPACK refuses one: INFO = -1.
      info = -1
      order_r = 0.0d0
      return
    end if

    allocate(theta(n), dtheta(n), stat=stat)
    if (stat /= 0) then
      order_r = 0.0d0
      return
    end if

    theta = phases_in
    k_over_n = k / real(n, c_double)

    do s = 1, steps
      do i = 1, n
        dtheta(i) = omegas(i)
        do j = 1, n
          dtheta(i) = dtheta(i) + k_over_n * sin(theta(j) - theta(i))
        end do
      end do
      theta = theta + dt * dtheta
    end do

    ! Order parameter r = |(1/N) Σ e^(iθ)|
    cos_sum = 0.0d0
    sin_sum = 0.0d0
    do i = 1, n
      cos_sum = cos_sum + cos(theta(i))
      sin_sum = sin_sum + sin(theta(i))
    end do
    order_r = sqrt(cos_sum * cos_sum + sin_sum * sin_sum) / real(n, c_double)

    phases_out = theta
  end subroutine spectral_phase_lock

  ! LAPACK's workspace answer (WORK(1) after LWORK = -1) as a default
  ! integer, or stat = -1 when it won't fit one (or isn't a number).
  subroutine workspace_size(query, lwork, stat)
    real(c_double), intent(in) :: query
    integer, intent(out) :: lwork
    integer(c_int), intent(out) :: stat

    if (.not. (query <= real(huge(lwork), c_double))) then
      lwork = 0
      stat = -1
    else
      lwork = max(1, int(query))
      stat = 0
    end if
  end subroutine workspace_size

  ! The STAT= mechanism, probed: allocate `count` doubles and report the
  ! allocation's status. The tests' only way to make an ALLOCATE fail on
  ! purpose (the wrappers' sizes come from LAPACK's workspace query).
  subroutine spectral_allocate_probe(count, stat) &
      bind(c, name="spectral_allocate_probe")
    integer(c_int64_t), value, intent(in) :: count
    integer(c_int), intent(out) :: stat

    real(c_double), allocatable :: probe(:)

    allocate(probe(count), stat=stat)
    if (stat == 0) deallocate(probe)
  end subroutine spectral_allocate_probe

end module optics_spectral
